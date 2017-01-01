extern crate nom;
extern crate std;

use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Write, Result};
use std::net::{TcpStream, TcpListener};
use std::{thread, str};
use nom::IResult;
use grammar::http_message;
use api::*;
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

    fn read<R, F>(reader: &mut R, buffer: &mut Buffer, mut fun: F) -> Result<usize>
        where R: Read + Sized, F: FnMut(&mut R, Request) -> Result<usize> {
        let read = try!(buffer.from(reader));
        try!(buffer.read_from(|slice| {
            match http_message(slice) {
                IResult::Done(remainder, request) => {
                    try!(fun(reader, Request::from(request)));
                    Ok(slice.len() - remainder.len())
                },
                IResult::Incomplete(_) => {
                    Ok(0)
                },
                IResult::Error(err) => {
                    Err(Error::new(ErrorKind::Other, format!("{}", err)))
                },
            }
        }));
        Ok(read)
    }

    fn write<'a, W, H>(write: &mut W, handler: &mut H, request: Request<'a>) -> Result<usize>
        where W: Write + Sized, H: HttpHandler + Sized {
        let mut response = handler.handle(request);
        response.write_to(write)
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
                    match Server::read(&mut stream, &mut buffer, |s, request| {
                        let mut handler = FileHandler::new(std::env::current_dir().unwrap());
                        Server::write(s, &mut handler, request)
                    }) {
                        Ok(read) if read > 0 => { },
                        _ => break,
                    }
                }
            });
        }
        Ok(0)
    }
}


#[cfg(test)]
mod tests {
    use io::*;
    use grammar::*;
    use std::str;
    use api::Request;

    #[test]
    #[allow(unused_variables)]
    #[allow(unused_must_use)]
    fn read_supports_fragmentation() {
        let get = "GET / HTTP/1.1\r\n\r\n";
        let post = "POST /foo HTTP/1.1\r\n\r\n";
        let put = "PUT /bar HTTP/1.1\r\n\r\n";
        let option = "OPTION / HTTP/1.1\r\n\r\n";
        let index = vec!(get, post, put, option);
        let requests = index.iter().fold(String::new(), |a, &v| a + v);
        let data = requests.as_bytes();
        let mut buffer = Buffer::new(data.len());
        let mut read = Fragmented::new(data, 10);
        let mut count = 0;

        while count < index.len() {
            super::Server::read(&mut read, &mut buffer, |stream, message| {
                assert_eq!(message, Request::from(http_message(index[count].as_bytes()).unwrap().1));
                count += 1;
                Ok(1)
            });
        }
    }
}