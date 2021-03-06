use std::path::{Path};
use std::fs::{File, Metadata, canonicalize};
use std::io::{BufRead, Read, Write, Result};
use std::cmp::min;
use std::borrow::Cow;
use std::fmt;
use regex::Regex;
use crate::ast::*;
use crate::grammar::*;
use crate::parser::*;
use crate::io::*;


pub trait HttpHandler {
    fn handle<F>(&mut self, request: &mut Request, fun: F) -> Result<()>
        where F: FnMut(&mut Response) -> Result<()> + Sized;
}

pub trait WriteTo {
    fn write_to(&mut self, write: &mut dyn Write) -> Result<usize>;
}

pub struct FileHandler<T: AsRef<Path>> {
    base: T,
}

impl<T: AsRef<Path>> FileHandler<T> {
    pub fn new(base: T) -> FileHandler<T> {
        FileHandler {
            base,
        }
    }

    pub fn get(&self, path: &str) -> Result<Response> {
        let full_path = canonicalize(self.base.as_ref().join(&path[1..]))?;
        if !full_path.starts_with(&self.base) {
            return Ok(Response::unauthorized().message("Not allowed outside of base"));
        }
        let file: File = File::open(&full_path)?;
        let metadata: Metadata = file.metadata()?;
        if metadata.is_dir() {
            return Ok(Response::not_found().message("Path denotes a directory"));
        }
        Ok(Response::ok().
            content_type("text/plain".to_string()).
            content_length(metadata.len()).
            entity(MessageBody::Reader(Box::new(file))))
    }
}

impl<T: AsRef<Path>> HttpHandler for FileHandler<T> {
    fn handle<F>(&mut self, request: &mut Request, mut fun: F) -> Result<()>
        where F: FnMut(&mut Response) -> Result<()> + Sized {
        fun(&mut match *request {
            Request { method: "GET", uri: Uri { path, .. }, .. } => { self.get(path).unwrap_or_else(|_|Response::not_found().message("Not Found")) }
            _ => { Response::method_not_allowed() }
        })
    }
}

pub struct LogHandler<H> where H: HttpHandler {
    handler: H,
}

impl<H> LogHandler<H> where H: HttpHandler {
    pub fn new(handler: H) -> LogHandler<H> {
        LogHandler {
            handler,
        }
    }
}

impl<H> HttpHandler for LogHandler<H> where H: HttpHandler {
    fn handle<F>(&mut self, request: &mut Request, mut fun: F) -> Result<()>
        where F: FnMut(&mut Response) -> Result<()> + Sized {
        let r = format!("{}", request);
        self.handler.handle(request, |response| {
            println!("{}{}\n\n", r, response);
            fun(response)
        })
    }
}

#[derive(PartialEq, Debug)]
pub struct Uri<'a> {
    pub scheme: Option<&'a str>,
    pub authority: Option<&'a str>,
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub fragment: Option<&'a str>,
}

impl<'a> Uri<'a> {
    pub fn parse(value: &'a str) -> Uri<'a> {
        lazy_static! {
            static ref RFC3986: Regex = Regex::new("^(?:([^:/?\\#]+):)?(?://([^/?\\#]*))?([^?\\#]*)(?:\\?([^\\#]*))?(?:\\#(.*))?").unwrap();
        }

        let result = RFC3986.captures(value).unwrap();
        Uri {
            scheme: result.get(1).map(|s|s.as_str()),
            authority: result.get(2).map(|s|s.as_str()),
            path: result.get(3).unwrap().as_str(),
            query: result.get(4).map(|s|s.as_str()),
            fragment: result.get(5).map(|s|s.as_str()),
        }
    }
}

impl<'a> fmt::Display for Uri<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        if let Some(scheme) = self.scheme {
            write!(format, "{}:", scheme)?;
        }
        if let Some(authority) = self.authority {
            write!(format, "//{}", authority)?;
        }
        format.write_str(self.path)?;
        if let Some(query) = self.query {
            write!(format, "?{}", query)?;
        }
        if let Some(fragment) = self.fragment {
            write!(format, "#{}", fragment)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Debug)]
pub enum Message<'a> {
    Request(Request<'a>),
    Response(Response<'a>),
}

impl<'a> Message<'a> {
    pub fn parse(slice: &'a [u8]) -> Result<(Message<'a>, &'a [u8])> {
        result(http_message(slice)).map(|(message, remainder)| (Message::from(message), remainder))
    }

    pub fn read<R>(slice: &'a [u8], reader: &'a mut R) -> Result<(Message<'a>, usize)> where R: Read {
        result(message_head(slice)).map(move |(head, remainder)| {
            let head_length = slice.len() - remainder.len();
            let headers = head.headers;
            let (body, body_read) = MessageBody::read(&headers, remainder, reader);

            (match head.start_line {
                StartLine::RequestLine(line) => Message::Request(Request::new(line.method, line.request_target, headers, body)),
                StartLine::StatusLine(line) => Message::Response(Response::new(line.code, line.description, headers, body)),
            }, head_length + body_read)
        })
    }
}

impl<'a> From<HttpMessage<'a>> for Message<'a> {
    fn from(message: HttpMessage<'a>) -> Message<'a> {
        match message.start_line {
            StartLine::RequestLine(line) => Message::Request(Request::new(line.method, line.request_target, message.headers, message.body)),
            StartLine::StatusLine(line) => Message::Response(Response::new(line.code, line.description, message.headers, message.body)),
        }
    }
}

impl<'a> WriteTo for Message<'a> {
    fn write_to(&mut self, write: &mut dyn Write) -> Result<usize> {
        match *self {
            Message::Request(ref mut request) => request.write_to(write),
            Message::Response(ref mut response) => response.write_to(write),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Request<'a> {
    pub method: &'a str,
    pub uri: Uri<'a>,
    pub headers: Headers<'a>,
    pub entity: MessageBody<'a>,
}

impl<'a> Request<'a> {
    pub fn new(method: &'a str, url: &'a str, headers: Headers<'a>, entity: MessageBody<'a>) -> Request<'a> {
        Request { method, uri: Uri::parse(url), headers, entity }
    }

    pub fn request(method: &'a str, url: &'a str) -> Request<'a> {
        Request::new(method, url, Headers::new(), MessageBody::None)
    }

    pub fn parse(slice: &'a [u8]) -> Result<(Request<'a>, &'a [u8])> {
        result(http_message(slice)).map(|(request, remainder)| (Request::from(request), remainder))
    }

    pub fn get(url: &'a str) -> Request<'a> {
        Request::request("GET", url)
    }

    pub fn post(url: &'a str) -> Request<'a> {
        Request::request("POST", url)
    }

    pub fn put(url: &'a str) -> Request<'a> {
        Request::request("PUT", url)
    }

    pub fn delete(url: &'a str) -> Request<'a> {
        Request::request("DELETE", url)
    }

    pub fn option(url: &'a str) -> Request<'a> {
        Request::request("OPTION", url)
    }

    pub fn method(mut self, method: &'a str) -> Self {
        self.method = method;
        self
    }

    pub fn header<V>(mut self, name: &'a str, value: V) -> Self
        where V: Into<Cow<'a, str>> {
        self.headers.replace(name, value);
        self
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn remove_header(mut self, name: &str) -> Self {
        self.headers.remove(name);
        self
    }
}

impl<'a> From<HttpMessage<'a>> for Request<'a> {
    fn from(message: HttpMessage<'a>) -> Request<'a> {
        if let StartLine::RequestLine(line) = message.start_line {
            return Request::new(line.method, line.request_target, message.headers, message.body);
        }
        panic!("Can not convert HttpMessage that is a Response into a Request")
    }
}

impl<'a> fmt::Display for Request<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{}{}\r\n{}",
               RequestLine { method: self.method, request_target: self.uri.to_string().as_str(), version: HttpVersion { major: 1, minor: 1 } },
               self.headers,
               self.entity)
    }
}

impl<'a> WriteTo for Request<'a> {
    fn write_to(&mut self, write: &mut dyn Write) -> Result<usize> {
        let text = format!("{}{}\r\n",
                           RequestLine { method: self.method, request_target: self.uri.to_string().as_str(), version: HttpVersion { major: 1, minor: 1 } },
                           self.headers);
        let head = write.write(text.as_bytes())?;
        let body = self.entity.write_to(write)?;
        Ok(head + body)
    }
}

#[derive(PartialEq, Debug)]
pub struct Response<'a> {
    pub code: u16,
    pub description: &'a str,
    pub headers: Headers<'a>,
    pub entity: MessageBody<'a>,
}

impl<'a> Response<'a> {
    pub fn new(code: u16, description: &'a str, headers: Headers<'a>, entity: MessageBody<'a>) -> Response<'a> {
        Response { code, description, headers, entity }.build()
    }

    pub fn response(code: u16, description: &'a str) -> Response<'a> {
        Response::new(code, description, Headers::new(), MessageBody::None)
    }

    pub fn ok() -> Response<'a> {
        Response::response(200, "OK")
    }

    pub fn bad_request() -> Response<'a> {
        Response::response(400, "Bad Request")
    }

    pub fn unauthorized() -> Response<'a> {
        Response::response(401, "Unauthorized")
    }

    pub fn not_found() -> Response<'a> {
        Response::response(404, "Not Found")
    }

    pub fn method_not_allowed() -> Response<'a> {
        Response::response(405, "Method Not Allowed")
    }

    pub fn code(mut self, code: u16) -> Response<'a> {
        self.code = code;
        self
    }

    pub fn description(mut self, description: &'a str) -> Response<'a> {
        self.description = description;
        self
    }

    pub fn message(self, message: &'a str) -> Response<'a> {
        let bytes = message.as_bytes();
        self.description(message).
            content_type("text/plain".to_string()).
            entity(MessageBody::Slice(bytes))
    }

    pub fn header(mut self, name: &'a str, value: String) -> Response<'a> {
        self.headers.replace(name, value);
        self
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn remove_header(mut self, name: &str) -> Response<'a> {
        self.headers.remove(name);
        self
    }

    pub fn entity(mut self, entity: MessageBody<'a>) -> Response<'a> {
        self.entity = entity;
        self.build()
    }

    pub fn content_type(self, media_type: String) -> Response<'a> {
        self.header("Content-Type", media_type)
    }

    pub fn content_length(self, length: u64) -> Response<'a> {
        self.header("Content-Length", format!("{}", length))
    }

    fn calculate_length(&self) -> Option<u64> {
        match self.entity {
            MessageBody::None => { Some(0) }
            MessageBody::Slice(slice) => { Some(slice.len() as u64) }
            _ => None
        }
    }

    fn build(self) -> Response<'a> {
        if let Some(length) = self.calculate_length() {
            return self.content_length(length)
        }
        self
    }
}

impl<'a> From<HttpMessage<'a>> for Response<'a> {
    fn from(message: HttpMessage<'a>) -> Response<'a> {
        if let StartLine::StatusLine(line) = message.start_line {
            return Response::new(line.code, line.description, message.headers, message.body);
        }
        panic!("Can not convert HttpMessage that is a Request into a Response")
    }
}

impl<'a> fmt::Display for Response<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{}{}\r\n{}",
               StatusLine { code: self.code, description: self.description, version: HttpVersion { major: 1, minor: 1 } },
               self.headers,
               self.entity)
    }
}

impl<'a> WriteTo for Response<'a> {
    fn write_to(&mut self, write: &mut dyn Write) -> Result<usize> {
        let text = format!("{}{}\r\n",
                           StatusLine { code: self.code, description: self.description, version: HttpVersion { major: 1, minor: 1 } },
                           self.headers);
        let head = write.write(text.as_bytes())?;
        let body = self.entity.write_to(write)?;
        Ok(head + body)
    }
}

pub struct ChunkStream<R> where R: BufRead + Sized {
    pub read: R,
    pub state: ChunkStreamState,
}

#[derive(PartialEq, Debug)]
pub enum ChunkStreamState {
    NotStarted,
    Consumed(usize),
    Last(usize),
    Finished,
}

impl<R> ChunkStream<R> where R: BufRead + Sized {
    pub fn new(read: R) -> ChunkStream<R> {
        ChunkStream { read, state: ChunkStreamState::NotStarted }
    }

    pub fn update_state(&mut self) {
        match self.state {
            ChunkStreamState::Last(consumed) => {
                self.read.consume(consumed);
                self.state = ChunkStreamState::Finished;
            },
            ChunkStreamState::Consumed(consumed) => {
                self.read.consume(consumed)
            },
            _ => {}
        }
    }
}

impl<'a, R> Drop for ChunkStream<R> where R: BufRead + Sized {
    fn drop(&mut self) {
        while let Some(Ok(_)) = self.next() {}
    }
}

impl<'a, R> Streamer<'a> for ChunkStream<R> where R: BufRead + Sized {
    type Item = Result<Chunk<'a>>;

    fn next(&'a mut self) -> Option<Self::Item> {
        self.update_state();
        if self.state == ChunkStreamState::Finished {
            return None;
        }

        loop {
            let buffer = self.read.fill_buf().unwrap();
            if buffer.len() == 0 {
                self.state = ChunkStreamState::Finished;
                return None;
            }

            return match Chunk::read(buffer) {
                Ok((last @ Chunk::Last(..), consumed)) => {
                    self.state = ChunkStreamState::Last(consumed);
                    Some(Ok(last))
                },
                Ok((chunk, consumed)) => {
                    self.state = ChunkStreamState::Consumed(consumed);
                    Some(Ok(chunk))
                },
                Err(e) => Some(Err(e))
            };
        }
    }
}

impl<'a, R> Read for ChunkStream<R> where R: BufRead + Sized {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.next() {
            None => Ok(0),
            Some(Ok(Chunk::Slice(_, slice))) => {
                // TODO: handle when buf is too small
                let size = min(slice.len(), buf.len());
                buf[..size].copy_from_slice(slice);
                Ok(size)
            },
            Some(Ok(Chunk::Last(..))) => {
                Ok(0)
            },
            Some(Err(e)) => Err(e),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_uri() {
        let uri = Uri::parse("http://authority/some/path?query=string#fragment");
        assert_eq!(uri.scheme, Some("http"));
        assert_eq!(uri.authority, Some("authority"));
        assert_eq!(uri.path, "/some/path");
        assert_eq!(uri.query, Some("query=string"));
        assert_eq!(uri.fragment, Some("fragment"));
    }

    #[test]
    fn supports_relative() {
        let uri = Uri::parse("some/path");
        assert_eq!(uri.scheme, None);
        assert_eq!(uri.authority, None);
        assert_eq!(uri.path, "some/path");
        assert_eq!(uri.query, None);
        assert_eq!(uri.fragment, None);
    }

    #[test]
    fn supports_urns() {
        let uri = Uri::parse("uuid:720f11db-1a29-4a68-a034-43f80b27659d");
        assert_eq!(uri.scheme, Some("uuid"));
        assert_eq!(uri.authority, None);
        assert_eq!(uri.path, "720f11db-1a29-4a68-a034-43f80b27659d");
        assert_eq!(uri.query, None);
        assert_eq!(uri.fragment, None);
    }

    #[test]
    fn is_reverse_able() {
        let original = "http://authority/some/path?query=string#fragment";
        assert_eq!(Uri::parse(original).to_string(), original.to_string());
        let another = "some/path";
        assert_eq!(Uri::parse(another).to_string(), another.to_string());
    }

    #[test]
    fn can_pattern_match_a_request() {
        let request = Request::get("/some/path").header("Content-Type", "text/plain");
        match request {
            Request { method: "GET", uri: Uri { path: "/some/path", .. }, ref headers, .. } if headers.get("Content-Type") == Some("text/plain") => {},
            _ => {
                panic!("Should have matched");
            }
        }
    }

    #[test]
    fn can_parse_chunk_stream() {
        use std::io::BufRead;
        use crate::io::{BufferedRead, Streamer};
        use crate::ast::{Chunk, ChunkExtensions, Headers};

        let data = &b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nGET /new/request HTTP/1.1\r\n"[..];
        let buffered = BufferedRead::new(data);
        let mut stream = ChunkStream::new(buffered);
        if let Some(Ok(chunk)) = stream.next() {
            assert_eq!(chunk, Chunk::Slice(ChunkExtensions(vec![]), &b"Wiki"[..]));
        }
        if let Some(Ok(chunk)) = stream.next() {
            assert_eq!(chunk, Chunk::Slice(ChunkExtensions(vec![]), &b"pedia"[..]));
        }
        if let Some(Ok(chunk)) = stream.next() {
            assert_eq!(chunk, Chunk::Slice(ChunkExtensions(vec![]), &b" in\r\n\r\nchunks."[..]));
        }
        if let Some(Ok(chunk)) = stream.next() {
            assert_eq!(chunk, Chunk::Last(ChunkExtensions(vec![]), Headers::new()));
        }
        assert!(stream.next().is_none());

        let remainder = stream.read.fill_buf().unwrap();
        assert_eq!(remainder, &b"GET /new/request HTTP/1.1\r\n"[..]);
    }

    #[test]
    fn can_read_chunked_stream() {
        use std::io::{BufRead, Read};
        use crate::io::{BufferedRead};

        let data = &b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nGET /new/request HTTP/1.1\r\n"[..];
        let mut producer = BufferedRead::new(data);
        {
            let mut consumer = BufferedRead::new(ChunkStream::new(&mut producer));

            let mut result = String::new();
            consumer.read_to_string(&mut result).unwrap();
            assert_eq!(result, "Wikipedia in\r\n\r\nchunks.".to_owned());
        }
        {
            let remainder = producer.fill_buf().unwrap();
            assert_eq!(remainder, &b"GET /new/request HTTP/1.1\r\n"[..]);
        }
    }

    #[test]
    fn chunked_stream_always_reads_to_end() {
        use std::io::{BufRead};
        use crate::io::{BufferedRead};

        let data = &b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\nGET /new/request HTTP/1.1\r\n"[..];
        let mut producer = BufferedRead::new(data);
        {
            ChunkStream::new(&mut producer);
        }
        {
            let remainder = producer.fill_buf().unwrap();
            assert_eq!(remainder, &b"GET /new/request HTTP/1.1\r\n"[..]);
        }
    }
}