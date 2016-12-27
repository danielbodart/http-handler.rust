use std::ascii::AsciiExt;
use std::{fmt, str};
use std::io::{Write, Result};
use api::ToWrite;

#[derive(PartialEq, Debug)]
pub struct HttpVersion {
    pub major: u8,
    pub minor: u8,
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "HTTP/{}.{}", self.major, self.minor)
    }
}

#[derive(PartialEq, Debug)]
pub struct RequestLine<'a> {
    pub method: &'a str,
    pub request_target: &'a str,
    pub version: HttpVersion,
}

impl<'a> fmt::Display for RequestLine<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{} {} {}\r\n", self.method, self.request_target, self.version)
    }
}

#[derive(PartialEq, Debug)]
pub struct StatusLine<'a> {
    pub version: HttpVersion,
    pub code: u16,
    pub description: &'a str,
}

impl<'a> fmt::Display for StatusLine<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{} {} {}\r\n", self.version, self.code, self.description)
    }
}

#[derive(PartialEq, Debug)]
pub enum StartLine<'a> {
    RequestLine(RequestLine<'a>),
    StatusLine(StatusLine<'a>),
}

impl<'a> fmt::Display for StartLine<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StartLine::RequestLine(ref rl) => rl.fmt(format),
            StartLine::StatusLine(ref sl) => sl.fmt(format),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct Headers<'a> (pub Vec<(&'a str, String)>);

impl<'a> fmt::Display for Headers<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        let mut result = String::new();

        for &(name, ref value) in &self.0[0..self.0.len()] {
            result.push_str(format!("{}: {}\r\n", name, value).as_str());
        }

        write!(format, "{}", result)
    }
}

impl<'a> Headers<'a> {
    pub fn get(&'a self, name: &str) -> Option<&'a str> {
        (&self.0).into_iter()
            .find(|&&(key, _)| name.eq_ignore_ascii_case(key))
            .map(|&(_, ref value)| value.as_str())
    }

    pub fn content_length(&self) -> u64 {
        self.get("Content-Length").
            and_then(|value| value.parse().ok()).
            unwrap_or(0)
    }
}

#[derive(PartialEq, Debug)]
pub enum MessageBody<'a> {
    None,
    Slice(&'a [u8]),
    Vector(Vec<u8>),
}

impl<'a> fmt::Display for MessageBody<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageBody::Vector(ref vector) => {
                if let Ok(result) = String::from_utf8(vector.clone()) {
                    write!(format, "{}", result)
                } else {
                    Ok(())
                }
            },
            MessageBody::Slice(ref slice) => {
                if let Ok(result) = str::from_utf8(slice) {
                    write!(format, "{}", result)
                } else {
                    Ok(())
                }
            },
            MessageBody::None => Ok(()),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct HttpMessage<'a> {
    pub start_line: StartLine<'a>,
    pub headers: Headers<'a>,
    pub body: MessageBody<'a>,
}

impl<'a> fmt::Display for HttpMessage<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{}{}\r\n{}", self.start_line, self.headers, self.body)
    }
}

impl<'a> ToWrite for HttpMessage<'a> {
    fn to_write(&self, write: &mut Write) -> Result<usize> {
        let text = format!("{}", self);
        let bytes = text.as_bytes();
        let wrote = try!(write.write(bytes));
        assert_eq!(bytes.len(), wrote);
        Ok(wrote)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_version_display() {
        assert_eq!(format!("{}", HttpVersion { major: 1, minor: 1 }), "HTTP/1.1");
    }

    #[test]
    fn request_line_display() {
        assert_eq!(format!("{}", RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1, } }), "GET /where?q=now HTTP/1.1\r\n");
    }

    #[test]
    fn status_line_display() {
        assert_eq!(format!("{}", StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }), "HTTP/1.1 200 OK\r\n");
    }

    #[test]
    fn start_line_display() {
        assert_eq!(format!("{}", StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1, } })), "GET /where?q=now HTTP/1.1\r\n");
    }

    #[test]
    fn headers_display() {
        assert_eq!(format!("{}", Headers(vec!(("Content-Type", "plain/text".to_string()), ("Content-Length", "3".to_string())))), "Content-Type: plain/text\r\nContent-Length: 3\r\n");
    }

    #[test]
    fn message_body_display() {
        assert_eq!(format!("{}", MessageBody::Slice(&b"abc"[..])), "abc");
        assert_eq!(format!("{}", MessageBody::None), "");
    }

    #[test]
    fn http_message_display() {
        assert_eq!(format!("{}", HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "plain/text".to_string()), ("Content-Length", "3".to_string()))),
            body: MessageBody::Slice(&b"abc"[..]),
        }), "HTTP/1.1 200 OK\r\nContent-Type: plain/text\r\nContent-Length: 3\r\n\r\nabc");
    }

}