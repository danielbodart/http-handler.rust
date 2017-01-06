extern crate nom;
extern crate std;

use std::collections::HashMap;
use std::io::{Error, Read, Write, Result};
use std::net::{TcpStream, TcpListener};
use std::{thread, str};
use api::*;
use process::Process;
use io::*;

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
        let listener: TcpListener = TcpListener::bind(authority)?;
        self.port = listener.local_addr()?.port();
        println!("listening on http://{}:{}/", self.host, self.port);
        Ok(listener)
    }

    fn read<R, F>(reader: &mut R, buffer: &mut Buffer, mut fun: F) -> Result<usize>
        where R: Read + Sized, F: FnMut(Request) -> Result<usize> {
        let read = buffer.from(reader)?;
        buffer.read_from(|slice| {
            let (request, remainder) = Request::parse(slice)?;
            fun(request)?;
            Ok(slice.len() - remainder.len())
        })?;
        Ok(read)
    }

    fn write<'a, W, H>(write: &mut W, handler: &mut H, request: Request<'a>) -> Result<usize>
        where W: Write + Sized, H: HttpHandler + Sized {
        let mut response = handler.handle(request);
        response.write_to(write)
    }

    fn split(stream:Result<TcpStream>) -> Result<(TcpStream, TcpStream)> {
        let a = stream?;
        Ok((a.try_clone()?, a))
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
        let listener = self.listen()?;

        for stream in listener.incoming() {
            thread::spawn(|| {
                let (mut reader, mut writer) = Server::split(stream).unwrap();
                let mut buffer = Buffer::new(4096);
                loop {
                    match Server::read(&mut reader, &mut buffer, |request| {
                        let mut handler = FileHandler::new(std::env::current_dir().unwrap());
                        Server::write(&mut writer, &mut handler, request)
                    }) {
                        Ok(read) if read > 0 => {},
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
            super::Server::read(&mut read, &mut buffer, |message| {
                assert_eq!(message, Request::from(http_message(index[count].as_bytes()).unwrap().1));
                count += 1;
                Ok(1)
            });
        }
    }
}