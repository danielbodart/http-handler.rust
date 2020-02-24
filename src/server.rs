extern crate nom;
extern crate std;

use std::io::{Read, Result};
use std::net::{TcpStream, TcpListener};
use std::{thread, str};
use std::sync::Arc;
use std::marker::{Send};
use std::borrow::{Cow, Borrow};
use crate::api::*;
use crate::io::*;

pub struct Server<'a> {
    host: Cow<'a, str>,
    port: u16,
}

impl<'a> Server<'a> {
    pub fn new<H>(host: H, port: u16) -> Server<'a>
        where H: Into<Cow<'a, str>> {
        Server {
            host: host.into(),
            port: port,
        }
    }

    pub fn handler<F, H>(&mut self, fun: F) -> Result<()>
        where H: HttpHandler, F: Fn() -> Result<H> + Send + Sync + 'static {
        let listener = self.listen()?;
        let fun = Arc::new(fun);

        for stream in listener.incoming() {
            let fun = fun.clone();
            thread::spawn(move || -> Result<()> {
                let (mut reader, mut writer) = Stream::split(stream)?;
                let mut buffer = Buffer::with_capacity(4096);
                let mut handler = fun()?;
                loop {
                    match Stream::read(&mut reader, &mut buffer, |message| {
                        if let Message::Request(ref mut request) = *message {
                            return handler.handle(request, |response| {
                                consume(response.write_to(&mut writer))
                            });
                        }
                        Ok(())
                    }) {
                        Ok(()) => continue,
                        Err(e) => return Err(e),
                    }
                }
            });
        }
        Ok(())
    }

    fn listen(&mut self) -> Result<TcpListener> {
        let authority = (self.host.borrow(), self.port);
        let listener: TcpListener = TcpListener::bind(authority)?;
        self.port = listener.local_addr()?.port();
        println!("listening on http://{}:{}/", self.host, self.port);
        Ok(listener)
    }
}

pub struct Stream;

impl Stream {
    fn read<R, F>(reader: &mut R, buffer: &mut Buffer<Vec<u8>>, mut fun: F) -> Result<()>
        where R: Read + Sized, F: FnMut(&mut Message) -> Result<()> {
        consume(buffer.fill(reader))?;
        unit(buffer.read_from(|slice| {
            let (mut message, count) = Message::read(slice, reader)?;
            fun(&mut message)?;
            Ok(count)
        }))
    }

    fn split(stream: Result<TcpStream>) -> Result<(TcpStream, TcpStream)> {
        let a = stream?;
        Ok((a.try_clone()?, a))
    }
}

#[derive(Default)]
pub struct Client;

impl HttpHandler for Client {
    fn handle<F>(&mut self, request: &mut Request, mut fun: F) -> Result<()>
        where F: FnMut(&mut Response) -> Result<()> + Sized {
        let stream = TcpStream::connect(request.get_header("Host").unwrap());

        let (mut reader, mut writer) = Stream::split(stream)?;
        let mut buffer = Buffer::with_capacity(4096);

        request.write_to(&mut writer)?;

        Stream::read(&mut reader, &mut buffer, |message| {
            if let Message::Response(ref mut response) = *message {
                return fun(response)
            }
            Ok(())
        })
    }
}

#[cfg(test)]
#[allow(unused_variables)]
#[allow(unused_must_use)]
mod tests {
    use crate::io::*;
    
    use crate::api::*;

    #[test]
    fn read_supports_fragmentation() {
        let get = "GET / HTTP/1.1\r\n\r\n";
        let post = "POST /foo HTTP/1.1\r\n\r\n";
        let put = "PUT /bar HTTP/1.1\r\n\r\n";
        let option = "OPTION / HTTP/1.1\r\n\r\n";
        let index = vec!(get, post, put, option);
        let requests = index.iter().fold(String::new(), |a, &v| a + v);
        let data = requests.as_bytes();
        let mut buffer = Buffer::with_capacity(data.len());
        let mut read = Fragmented::new(data, 10);
        let mut count = 0;

        while count < index.len() {
            super::Stream::read(&mut read, &mut buffer, |message| {
                assert_eq!(*message, Message::parse(index[count].as_bytes()).unwrap().0);
                count += 1;
                Ok(())
            });
        }
    }

    #[test]
    fn read_handles_requests_that_fit_in_buffer() {
        let get = "GET / HTTP/1.1\r\n\r\n";
        let index = vec!(get, get, get, get);
        let requests = index.iter().fold(String::new(), |a, &v| a + v);
        let mut data = requests.as_bytes();
        let mut buffer = Buffer::with_capacity(get.len());
        let mut count = 0;

        while count < index.len() {
            super::Stream::read(&mut data, &mut buffer, |message| {
                assert_eq!(*message, Message::parse(index[count].as_bytes()).unwrap().0);
                count += 1;
                Ok(())
            }).expect("No errors");
        }
    }

    #[test]
    fn read_handles_requests_that_head_fits_in_buffer_but_body_is_streamed() {
        let head = "POST /where?q=now HTTP/1.1\r\nContent-Type: plain/text\r\nContent-Length: 26\r\n\r\n";
        let body = "abcdefghijklmnopqrstuvwxyz";
        let request = head.to_owned() + body;
        let index = vec!(head, body, head, body);
        let requests = index.iter().fold(String::new(), |a, &v| a + v);
        let mut data = requests.as_bytes();
        let mut buffer = Buffer::with_capacity(head.len());
        let mut count = 0;

        super::Stream::read(&mut data, &mut buffer, |message| {
            let mut result = String::new();
            unsafe { message.write_to(result.as_mut_vec()) };
            assert_eq!(result, request);
            count += 1;
            Ok(())
        }).expect("No errors");

        assert_eq!(count, 1);

        super::Stream::read(&mut data, &mut buffer, |message| {
            let mut result = String::new();
            unsafe { message.write_to(result.as_mut_vec()) };
            assert_eq!(result, request);
            count += 1;
            Ok(())
        }).expect("No errors");

        assert_eq!(count, 2);


        assert!(super::Stream::read(&mut data, &mut buffer, |message| {
            panic!("Should not be any more data")
        }).is_err());
    }

    #[test]
    fn read_handles_requests_where_the_body_is_not_consumed() {
        let head = "POST /where?q=now HTTP/1.1\r\nContent-Type: plain/text\r\nContent-Length: 26\r\n\r\n";
        let body = "abcdefghijklmnopqrstuvwxyz";
        let request = head.to_owned() + body;
        let index = vec!(head, body, head, body);
        let requests = index.iter().fold(String::new(), |a, &v| a + v);
        let mut data = requests.as_bytes();
        let mut buffer = Buffer::with_capacity(head.len());
        let mut count = 0;

        super::Stream::read(&mut data, &mut buffer, |message| {
            // Ignore message so body is not consumed
            count += 1;
            Ok(())
        }).expect("No errors");

        assert_eq!(count, 1);

        super::Stream::read(&mut data, &mut buffer, |message| {
            // Ignore message so body is not consumed
            count += 1;
            Ok(())
        }).expect("No errors");

        assert_eq!(count, 2);


        assert!(super::Stream::read(&mut data, &mut buffer, |message| {
            panic!("Should not be any more data")
        }).is_err());
    }
}