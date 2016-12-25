extern crate nom;
extern crate std;

use std::collections::HashMap;
use std::io::{Error, Read, Write, ErrorKind};
use std::net::{TcpStream, TcpListener};
use std::thread;
use nom::IResult;
use grammar::http_message;
use ast::*;
use api::{ToWrite, HttpHandler};
use process::Process;

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

    fn listen(&mut self) -> Result<TcpListener, Error> {
        let authority = (self.host.as_str(), self.port);
        let listener: TcpListener = try!(TcpListener::bind(authority));
        self.port = try!(listener.local_addr()).port();
        println!("listening on http://{}:{}/", self.host, self.port);
        Ok(listener)
    }

    fn read<'a, R>(read: &mut R, buffer: &'a mut [u8]) -> Result<HttpMessage<'a>, Error>
        where R: Read + Sized {
        let read = read.read(&mut buffer[..]).unwrap();
        match http_message(&buffer[..read]) {
            IResult::Done(_, request) => {
                Ok(request)
            },
            IResult::Incomplete(needed) => {
                Err(Error::new(ErrorKind::Other, format!("Incomplete need {:?}", needed)))
            },
            IResult::Error(err) => {
                Err(Error::new(ErrorKind::Other, format!("Error {}", err)))
            },
        }
    }

    fn write<'a, W, H>(write: &mut W, handler: &mut H, request: &HttpMessage<'a>) where W: Write + Sized, H: HttpHandler + Sized {
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
    fn run(&mut self) -> Result<i32, Error> {
        let listener = try!(self.listen());

        for stream in listener.incoming() {
            thread::spawn(|| {
                let mut stream: TcpStream = stream.unwrap();
                let mut buffer: [u8; 4096] = [0; 4096];
                match Server::read(&mut stream, &mut buffer) {
                    Ok(request) => {
                        let mut handler = LogHandler { handler: TestHandler {} };
                        Server::write(&mut stream, &mut handler, &request)
                    },
                    Err(error) => {
                        println!("{}", error)
                    }
                }
            });
        }
        Ok(0)
    }
}


struct TestHandler {}

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
    use ast::*;
    use std::io::{Read, Cursor, Result};

    #[test]
    fn read_supports_fragmentation() {
        let request = b"GET / HTTP/1.1\r\n\r\n";
        let mut buffer: [u8; 32] = [0; 32];

        struct Fragmented<'a> {
            data: Vec<&'a [u8]>,
            count: usize,
        }

        impl<'a> Read for Fragmented<'a> {
            fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
                if self.data.len() == self.count {
                    return Ok(0)
                }
                let fragment = self.data[self.count];
                buf[..fragment.len()].copy_from_slice(fragment);

                self.count += 1;
                Ok(fragment.len())
            }
        }

        let mut read = Fragmented {
            data: vec!(&request[..7], &request[8..]),
            count: 0,
        };

        assert_eq!(super::Server::read(&mut read, &mut buffer).unwrap(), HttpMessage {
            start_line: StartLine::RequestLine(RequestLine { method: "GET", request_target: "/", version: HttpVersion { major: 1, minor: 1, } }),
            headers: Headers(Vec::new()),
            body: MessageBody::None,
        });
    }
}