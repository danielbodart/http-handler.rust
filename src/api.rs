use std::borrow::Cow;
use std::path::{Path};
use std::fs::{File, Metadata, canonicalize};
use std::io::{Error, ErrorKind, Write, Result};
use std::fmt;
use regex::Regex;
use ast::*;


pub trait HttpHandler {
    fn handle(&mut self, request: &Request) -> HttpMessage;
}

pub trait WriteTo {
    fn write_to(&mut self, write: &mut Write) -> Result<usize>;
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
        let file: File = try!(File::open(&full_path));
        let metadata: Metadata = try!(file.metadata());
        if metadata.is_dir() {
            return Err(Error::new(ErrorKind::NotFound, "Path denotes a directory"));
        }
        Ok(HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", format!("{}", metadata.len())))),
            body: MessageBody::Reader(Box::new(file)),
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
    fn handle(&mut self, request: &Request) -> HttpMessage {
        match *request {
            Request { method: "GET", uri: Uri { path, .. }, .. } => { return self.get(path).unwrap_or(self.not_found()) }
            _ => { self.not_found() }
        }
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
    fn handle(&mut self, request: &Request) -> HttpMessage {
        let response = self.handler.handle(request);
        print!("{}{}\n\n\n", request, response);
        response
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
            scheme: result.at(1),
            authority: result.at(2),
            path: result.at(3).unwrap(),
            query: result.at(4),
            fragment: result.at(5),
        }
    }

    pub fn to_string(&self) -> String {
        let mut builder = String::new();
        if let Some(scheme) = self.scheme {
            builder = builder + scheme + ":";
        }
        if let Some(authority) = self.authority {
            builder = builder + "//" + authority;
        }
        builder += self.path;
        if let Some(query) = self.query {
            builder = builder + "?" + query;
        }
        if let Some(fragment) = self.fragment {
            builder = builder + "#" + fragment;
        }
        return builder;
    }
}

impl<'a> fmt::Display for Uri<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{}", self.to_string())
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
        Request { method: method, uri: Uri::parse(url), headers: headers, entity: entity }
    }

    pub fn request(method: &'a str, url: &'a str) -> Request<'a> {
        Request::new(method, url, Headers::new(), MessageBody::None)
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

    pub fn method(&mut self, method: &'a str) -> &mut Request<'a> {
        self.method = method;
        self
    }

    pub fn header(&mut self, name: &'a str, value: &str) -> &mut Request<'a> {
        self.headers.replace(name, value);
        self
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn remove_header(&mut self, name: &str) -> &mut Request<'a> {
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

#[derive(PartialEq, Debug)]
pub struct Response<'a> {
    pub code: u16,
    pub description: &'a str,
    pub headers: Headers<'a>,
    pub entity: MessageBody<'a>,
}

impl<'a> Response<'a> {
    pub fn new(code: u16, description: &'a str, headers: Headers<'a>, entity: MessageBody<'a>) -> Response<'a> {
        Response { code: code, description: description, headers: headers, entity: entity }
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

    pub fn code(&mut self, code: u16) -> &mut Response<'a> {
        self.code = code;
        self
    }

    pub fn description(&mut self, description: &'a str) -> &mut Response<'a> {
        self.description = description;
        self
    }

    pub fn header(&mut self, name: &'a str, value: &str) -> &mut Response<'a> {
        self.headers.replace(name, value);
        self
    }

    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)
    }

    pub fn remove_header(&mut self, name: &str) -> &mut Response<'a> {
        self.headers.remove(name);
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

#[cfg(test)]
mod tests {
    use super::{Request, Uri};

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
        let request = Request::get("/some/path");
        match request {
            Request { method: "GET", uri: Uri { path: "/some/path", .. }, .. } => {},
            _ => {
                panic!("Should have matched");
            }
        }
    }
}