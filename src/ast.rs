use std::ascii::AsciiExt;
use std::{fmt, str, usize};
use std::io::{Read, Write, Result, copy};
use api::{WriteTo};

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
        for &(name, ref value) in &self.0[0..self.0.len()] {
            write!(format, "{}: {}\r\n", name, value)?;
        }
        Ok(())
    }
}

impl<'a> Headers<'a> {
    pub fn new() -> Headers<'a> {
        Headers(vec!())
    }

    pub fn get(&'a self, name: &str) -> Option<&'a str> {
        self.pairs().into_iter()
            .find(|&&(key, _)| name.eq_ignore_ascii_case(key))
            .map(|&(_, ref value)| value.as_str())
    }

    fn pairs(&'a self) -> &Vec<(&'a str, String)> {
        &self.0
    }

    pub fn content_length(&self) -> u64 {
        self.get("Content-Length").
            and_then(|value| value.parse().ok()).
            unwrap_or(0)
    }

    pub fn replace(&mut self, name: &'a str, value: String) -> &mut Headers<'a> {
        self.0.retain(|&(key, _)| !name.eq_ignore_ascii_case(key));
        self.0.push((name, value));
        self
    }

    pub fn remove(&mut self, name: &str) -> &mut Headers<'a> {
        self.0.retain(|&(key, _)| !name.eq_ignore_ascii_case(key));
        self
    }
}

type SizedRead = Read + Sized;

pub enum MessageBody<'a> {
    None,
    Slice(&'a [u8]),
    Reader(Box<Read>),
}

impl<'a> MessageBody<'a> {
    fn format(&self, format: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageBody::Reader(_) => {
                format.write_str("streaming")
            },
            MessageBody::Slice(ref slice) => {
                if let Ok(result) = str::from_utf8(slice) {
                    format.write_str(result)
                } else {
                    Ok(())
                }
            },
            MessageBody::None => Ok(()),
        }
    }
}

impl<'a> PartialEq for MessageBody<'a> {
    fn eq(&self, other: &MessageBody) -> bool {
        match (self, other) {
            (&MessageBody::None, &MessageBody::None) => true,
            (&MessageBody::Slice(ref slice_a), &MessageBody::Slice(ref slice_b)) => slice_a == slice_b,
            (&MessageBody::Reader(_), &MessageBody::Reader(_)) => true,
            _ => false
        }
    }
}

impl<'a> fmt::Display for MessageBody<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        self.format(format)
    }
}

impl<'a> fmt::Debug for MessageBody<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        self.format(format)
    }
}

impl<'a> WriteTo for MessageBody<'a> {
    fn write_to(&mut self, writer: &mut Write) -> Result<usize> {
        match *self {
            MessageBody::Reader(ref mut reader) => {
                copy(reader, writer).map(|c| {
                    if c > usize::MAX as u64 {
                        usize::MAX
                    } else {
                        c as usize
                    }
                })
            },
            MessageBody::Slice(ref slice) => {
                writer.write(&slice)
            },
            MessageBody::None => Ok(0),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct MessageHead<'a> {
    pub start_line: StartLine<'a>,
    pub headers: Headers<'a>,
}

impl<'a> fmt::Display for MessageHead<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        write!(format, "{}{}\r\n", self.start_line, self.headers)
    }
}

impl<'a> WriteTo for MessageHead<'a> {
    fn write_to(&mut self, write: &mut Write) -> Result<usize> {
        let text = format!("{}{}\r\n", self.start_line, self.headers);
        let head = write.write(text.as_bytes())?;
        Ok(head)
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

impl<'a> WriteTo for HttpMessage<'a> {
    fn write_to(&mut self, write: &mut Write) -> Result<usize> {
        let text = format!("{}{}\r\n", self.start_line, self.headers);
        let head = write.write(text.as_bytes())?;
        let body = self.body.write_to(write)?;
        Ok(head + body)
    }
}

#[derive(PartialEq, Debug)]
pub struct ChunkExtensions<'a> (pub Vec<(&'a str, Option<String>)>);

impl<'a> fmt::Display for ChunkExtensions<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        for &(name, ref option) in &self.0[0..self.0.len()] {
            if let Some(ref value) = *option {
                write!(format, ";{}={}", name, value)?;
            } else {
                write!(format, ";{}", name)?;
            }
        }
        Ok(())
    }
}

#[derive(PartialEq, Debug)]
pub enum Chunk<'a> {
    Slice(ChunkExtensions<'a>, &'a [u8]),
    Last(ChunkExtensions<'a>),
}

#[derive(PartialEq, Debug)]
pub struct ChunkedBody<'a> {
    chunks: Vec<Chunk<'a>>,
    trailers: Headers<'a>
}

impl<'a> ChunkedBody<'a> {
    pub fn new(mut chunks:Vec<Chunk<'a>>, last:Chunk<'a>, trailers:Headers<'a>) -> ChunkedBody<'a> {
        chunks.push(last);
        ChunkedBody {
            chunks: chunks,
            trailers: trailers,
        }
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