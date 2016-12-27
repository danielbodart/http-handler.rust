extern crate nom;
extern crate std;

use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Write, Result};
use std::net::{TcpStream, TcpListener};
use std::{thread, str};
use nom::IResult;
use grammar::http_message;
use ast::*;
use api::{ToWrite, HttpHandler};
use process::Process;
use io::Buffer;

pub struct Server {
    port: u16,
    host: String,
}

impl Server {
    fn port(env: &HashMap<String, String>) -> u16 {
        env.get("PORT").and_then(|value| value.parse().ok()).unwrap_or(8080)
    }

    fn host(env: &HashMap<String, String>) -> String {
        env.get("HOST").unwrap_or(&"0.0.0.0".to_string()).clone()
    }

    fn listen(&mut self) -> Result<TcpListener> {
        let authority = (self.host.as_str(), self.port);
        let listener: TcpListener = try!(TcpListener::bind(authority));
        self.port = try!(listener.local_addr()).port();
        println!("listening on http://{}:{}/", self.host, self.port);
        Ok(listener)
    }

    fn read<R, F>(read: &mut R, buffer: &mut Buffer, mut fun: F) -> Result<()>
        where R: Read + Sized, F: FnMut(&mut R, HttpMessage) -> () {
        try!(buffer.from(read));
        try!(buffer.read_from(|slice| {
            match http_message(slice) {
                IResult::Done(remainder, request) => {
                    fun(read, request);
                    Ok(slice.len() - remainder.len())
                },
                _ => {
                    Err(Error::new(ErrorKind::Other, "Failed to read request"))
                },
            }
        }));
        Ok(())
    }

    #[allow(unused_must_use)]
    fn write<'a, W, H>(write: &mut W, handler: &mut H, request: &HttpMessage<'a>)
        where W: Write + Sized, H: HttpHandler + Sized {
        let response = handler.handle(&request);
        response.to_write(write);
    }
}


impl Process<Error> for Server {
    fn new(args: Vec<String>, env: HashMap<String, String>) -> Server {
        assert_eq!(args.len(), 1);

        Server {
            port: Server::port(&env),
            host: Server::host(&env),
        }
    }
    fn run(&mut self) -> Result<i32> {
        let listener = try!(self.listen());

        for stream in listener.incoming() {
            thread::spawn(|| {
                let mut stream: TcpStream = stream.unwrap();
                let mut buffer = Buffer::new(4096);
                loop {
                    Server::read(&mut stream, &mut buffer, |stream, request| {
                        let mut handler = LogHandler { handler: TestHandler {} };
                        Server::write(stream, &mut handler, &request);
                    }).expect("Error while reading stream");
                }
            });
        }
        Ok(0)
    }
}


struct TestHandler {}

#[allow(unused_variables)]
impl HttpHandler for TestHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", "5".to_string()))),
            body: MessageBody::Slice(&b"Hello"[..]),
        }
    }
}

struct LogHandler<H> where H: HttpHandler {
    handler: H,
}

impl<H> HttpHandler for LogHandler<H> where H: HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        let response = self.handler.handle(request);
        print!("{}{}\n\n\n", request, response);
        response
    }
}

#[cfg(test)]
mod tests {
    use io::*;
    use grammar::*;
    use std::str;

    #[test]
    #[allow(unused_variables)]
    #[allow(unused_must_use)]
    fn read_supports_fragmentation() {
        let get = "GET / HTTP/1.1\r\n\r\n";
        let post = "POST /foo HTTP/1.1\r\n\r\n";
        let put = "PUT /bar HTTP/1.1\r\n\r\n";
        let option = "OPTION / HTTP/1.1\r\n\r\n";
        let index = vec!(get, post, put, option);
        let requests = format!("{}{}{}{}", get, post, put, option);
        let data = requests.as_bytes();
        let mut buffer = Buffer::new(data.len());
        let mut read = Fragmented::new(data, 10);
        let mut count = 0;

        while count < index.len() {
            super::Server::read(&mut read, &mut buffer, |stream, message|{
                let http = index[count];
                assert_eq!(message, http_message(http.as_bytes()).unwrap().1);
                count += 1;
            });
        }
    }
}