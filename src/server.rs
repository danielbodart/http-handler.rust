extern crate nom;
extern crate std;

use std::io::{Read, Write, Result};
use std::net::{TcpStream, TcpListener};
use std::{thread, str};
use std::sync::Arc;
use std::marker::{Send};
use api::*;
use io::*;

pub struct Server {
    host: String,
    port: u16,
}

impl Server {
    pub fn new(host:String, port:u16) -> Server {
        Server {
            host: host,
            port: port,
        }
    }

    pub fn handler<F, H>(&mut self, fun:F) -> Result<()>
        where H:HttpHandler, F:Fn() -> Result<H> + Send + Sync + 'static{
        let listener = self.listen()?;
        let fun = Arc::new(fun);

        for stream in listener.incoming() {
            let fun = fun.clone();
            thread::spawn(move || -> Result<()> {
                let (mut reader, mut writer) = Server::split(stream)?;
                let mut buffer = Buffer::with_capacity(4096);
                let mut handler = fun()?;
                loop {
                    match Server::read(&mut reader, &mut buffer, |mut request| {
                        Server::write(&mut writer, &mut handler, &mut request)
                    }) {
                        Ok(read) if read > 0 => continue,
                        _ => return Ok(()),
                    }
                }
            });
        }
        Ok(())

    }

    fn listen(&mut self) -> Result<TcpListener> {
        let authority = (self.host.as_str(), self.port);
        let listener: TcpListener = TcpListener::bind(authority)?;
        self.port = listener.local_addr()?.port();
        println!("listening on http://{}:{}/", self.host, self.port);
        Ok(listener)
    }

    fn read<R, F>(reader: &mut R, buffer: &mut Buffer, mut fun: F) -> Result<usize>
        where R: Read + Sized, F: FnMut(&mut Request) -> Result<usize> {
        let read = buffer.from(reader)?;
        buffer.read_from(|slice| {
            let (mut request, count) = Request::read(slice, reader)?;
            fun(&mut request)?;
            request.entity.drain()?;
            Ok(count)
        })?;
        Ok(read)
    }

    fn write<'a, W, H>(write: &mut W, handler: &mut H, request: &mut Request<'a>) -> Result<usize>
        where W: Write + Sized, H: HttpHandler + Sized {
        let mut response = handler.handle(request);
        response.write_to(write)
    }

    fn split(stream: Result<TcpStream>) -> Result<(TcpStream, TcpStream)> {
        let a = stream?;
        Ok((a.try_clone()?, a))
    }
}



#[cfg(test)]
#[allow(unused_variables)]
#[allow(unused_must_use)]
mod tests {
    use io::*;
    use grammar::*;
    use std::str;
    use api::*;

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
            super::Server::read(&mut read, &mut buffer, |message| {
                assert_eq!(*message, Request::from(http_message(index[count].as_bytes()).unwrap().1));
                count += 1;
                Ok(1)
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
            super::Server::read(&mut data, &mut buffer, |message| {
                assert_eq!(*message, Request::from(http_message(index[count].as_bytes()).unwrap().1));
                count += 1;
                Ok(1)
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

        super::Server::read(&mut data, &mut buffer, |message| {
            let mut result = String::new();
            unsafe {message.write_to(result.as_mut_vec())};
            assert_eq!(result, request);
            count += 1;
            Ok(1)
        }).expect("No errors");

        assert_eq!(count, 1);

        super::Server::read(&mut data, &mut buffer, |message| {
            let mut result = String::new();
            unsafe {message.write_to(result.as_mut_vec())};
            assert_eq!(result, request);
            count += 1;
            Ok(1)
        }).expect("No errors");

        assert_eq!(count, 2);


        assert!(super::Server::read(&mut data, &mut buffer, |message| {
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

        super::Server::read(&mut data, &mut buffer, |message| {
            // Ignore message so body is not consumed
            count += 1;
            Ok(1)
        }).expect("No errors");

        assert_eq!(count, 1);

        super::Server::read(&mut data, &mut buffer, |message| {
            // Ignore message so body is not consumed
            count += 1;
            Ok(1)
        }).expect("No errors");

        assert_eq!(count, 2);


        assert!(super::Server::read(&mut data, &mut buffer, |message| {
            panic!("Should not be any more data")
        }).is_err());
    }
}