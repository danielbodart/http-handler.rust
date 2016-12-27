use std::borrow::Cow;
use std::path::{Path};
use std::fs::{File, canonicalize};
use std::io::{Error, ErrorKind, Read, Write, Result};
use regex::Regex;
use ast::*;


pub trait HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage;
}

pub trait ToWrite {
    fn to_write(&self, write: &mut Write) -> Result<usize>;
}

pub struct FileHandler<'a> {
    base: Cow<'a, Path>,
}

impl<'a> FileHandler<'a> {
    pub fn new<P>(base: P) -> FileHandler<'a>
        where P: Into<Cow<'a, Path>> {
        FileHandler {
            base: base.into(),
        }
    }

    pub fn get(&self, path: &str) -> Result<HttpMessage> {
        let full_path = try!(canonicalize(self.base.join(&path[1..])));
        if !full_path.starts_with(&self.base) {
            return Err(Error::new(ErrorKind::PermissionDenied, "Not allowed outside of base"));
        }
        let mut file: File = try!(File::open(&full_path));
        let mut buffer = Vec::new();
        let count = try!(file.read_to_end(&mut buffer));
        Ok(HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", format!("{}", count)))),
            body: MessageBody::Vector(buffer),
        })
    }

    pub fn not_found(&self) -> HttpMessage {
        HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 404, description: "Not Found" }),
            headers: Headers(vec!(("Content-Length", "0".to_string()))),
            body: MessageBody::None,
        }
    }
}

impl<'a> HttpHandler for FileHandler<'a> {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        if let StartLine::RequestLine(RequestLine { method: "GET", request_target: uri, version: _ }) = request.start_line {
            return self.get(Uri::parse(uri).path).unwrap_or(self.not_found());
        }
        self.not_found()
    }
}

pub struct LogHandler<H> where H: HttpHandler {
    handler: H,
}

impl<H> LogHandler<H> where H: HttpHandler {
    pub fn new(handler: H) -> LogHandler<H> {
        LogHandler {
            handler: handler,
        }
    }
}

impl<H> HttpHandler for LogHandler<H> where H: HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        let response = self.handler.handle(request);
        print!("{}{}\n\n\n", request, response);
        response
    }
}

pub struct Uri<'a> {
    pub scheme: &'a str,
    pub authority: &'a str,
    pub path: &'a str,
    pub query: &'a str,
    pub fragment: &'a str,
}

impl<'a> Uri<'a> {
    pub fn parse(value: &'a str) -> Uri<'a> {
        lazy_static! {
            static ref RFC3986: Regex = Regex::new("^(?:([^:/?\\#]+):)?(?://([^/?\\#]*))?([^?\\#]*)(?:\\?([^\\#]*))?(?:\\#(.*))?").unwrap();
        }

        let result = RFC3986.captures(value).unwrap();
        Uri {
            scheme: result.at(1).unwrap_or(""),
            authority: result.at(2).unwrap_or(""),
            path: result.at(3).unwrap_or(""),
            query: result.at(4).unwrap_or(""),
            fragment: result.at(5).unwrap_or(""),
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn can_parse_uri() {
        let uri = super::Uri::parse("http://authority/some/path?query=string#fragment");
        assert_eq!(uri.scheme, "http");
        assert_eq!(uri.authority, "authority");
        assert_eq!(uri.path, "/some/path");
        assert_eq!(uri.query, "query=string");
        assert_eq!(uri.fragment, "fragment");
    }

    #[test]
    fn supports_relative() {
        let uri = super::Uri::parse("some/path");
        assert_eq!(uri.scheme, "");
        assert_eq!(uri.authority, "");
        assert_eq!(uri.path, "some/path");
        assert_eq!(uri.query, "");
        assert_eq!(uri.fragment, "");
    }
}