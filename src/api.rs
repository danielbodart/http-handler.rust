use std::borrow::Cow;
use std::path::{Path};
use std::fs::{File, canonicalize};
use std::io::{Error, ErrorKind, Read, Write, Result};
use ast::*;

pub trait HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage;
}

pub trait ToWrite {
    fn to_write(&self, write:&mut Write) -> Result<usize>;
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
            headers: Headers(vec!()),
            body: MessageBody::None,
        }
    }
}

impl<'a> HttpHandler for FileHandler<'a> {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        if let StartLine::RequestLine(RequestLine { method: "GET", request_target: path, version: _ }) = request.start_line {
            // TODO parse URL correctly to extract path
            return self.get(path).unwrap_or(self.not_found());
        }
        self.not_found()
    }
}

pub struct LogHandler<H> where H: HttpHandler {
    handler: H,
}

impl <H> LogHandler<H> where H: HttpHandler {
    pub fn new(handler:H) -> LogHandler<H> {
        LogHandler {
            handler:handler,
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