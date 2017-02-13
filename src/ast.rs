use std::ascii::AsciiExt;
use std::{fmt, str, usize};
use std::io::{Read, Write, Result, copy, sink};
use api::{WriteTo};
use std::borrow::{Cow, Borrow};
use parser::result;
use nom::IResult;
use io::SimpleError;

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
pub struct Header<'a> {
    pub name: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

impl<'a> Header<'a> {
    pub fn new<N, V>(name: N, value: V) -> Header<'a>
        where N: Into<Cow<'a, str>>,
              V: Into<Cow<'a, str>> {
        Header { name: name.into(), value: value.into() }
    }

    pub fn name(&self) -> &str {
        self.name.borrow()
    }

    pub fn value(&self) -> &str {
        self.value.borrow()
    }
}

#[derive(PartialEq, Debug)]
pub struct Headers<'a> (pub Vec<Header<'a>>);


impl<'a> fmt::Display for Headers<'a> {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        for header in &self.0[0..self.0.len()] {
            write!(format, "{}: {}\r\n", header.name(), header.value())?;
        }
        Ok(())
    }
}

type NomParser<'a, T> = fn(&'a[u8]) -> IResult<&'a[u8], Vec<T>>;


impl<'a> Headers<'a> {
    pub fn new() -> Headers<'a> {
        Headers(vec!())
    }

    pub fn get(&'a self, name: &str) -> Option<&'a str> {
        (&self.0).into_iter().
            find(|header| name.eq_ignore_ascii_case(header.name())).
            map(|header| header.value())
    }

    pub fn headers(&'a self, name: &str) -> Vec<&'a str> {
        (&self.0).into_iter().
            filter(|header| name.eq_ignore_ascii_case(header.name())).
            map(|header| header.value()).
            collect()
    }

    pub fn parse<F, T>(&'a self, name: &str, fun: F) -> Result<Vec<T>>
        where T: 'a, F: Fn(&'a str) -> Result<Vec<T>> {
        let mut result = Vec::new();
        for header in self.headers(name) {
            result.extend(fun(header)?);
        }
        Ok(result)
    }

    pub fn parse_nom<T>(&'a self, name: &str, fun: NomParser<'a, T>) -> Result<Vec<T>>
        where T: 'a {
        self.parse(name, |s| {
            let (encodings, remainder) = result(fun(s.as_bytes()))?;
            if !remainder.is_empty() {
                return Err(SimpleError::error(format!("Invalid header '{}' has remainder '{}'", name, str::from_utf8(remainder).unwrap())))
            }
            Ok(encodings)
        })
    }

    pub fn transfer_encoding(&'a self) -> Vec<TransferCoding<'a>>{
        use grammar::transfer_encoding;

        self.parse_nom("Transfer-Encoding", transfer_encoding).unwrap_or_else(|_|Default::default())
    }

    pub fn content_length(&self) -> Option<u64> {
        self.get("Content-Length").
            and_then(|value| value.parse().ok())
    }

    pub fn replace<V>(&mut self, name: &'a str, value: V) -> &mut Headers<'a>
        where V: Into<Cow<'a, str>> {
        self.0.retain(|header| !name.eq_ignore_ascii_case(header.name()));
        self.0.push(Header::new(name, value));
        self
    }

    pub fn remove(&mut self, name: &str) -> &mut Headers<'a> {
        self.0.retain(|header| !name.eq_ignore_ascii_case(header.name()));
        self
    }
}

pub enum MessageBody<'a> {
    None,
    Slice(&'a [u8]),
    Reader(Box<Read + 'a>),
}

impl<'a> MessageBody<'a> {
    pub fn read<R>(headers: &Headers, slice: &'a [u8], reader: &'a mut R) -> (MessageBody<'a>, usize) where R: Read {
        match headers.content_length() {
            Some(body_length) if body_length > 0 => {
                let slice_length = slice.len() as u64;
                if body_length <= slice_length {
                    let length = body_length as usize;
                    (MessageBody::Slice(&slice[..length]), length)
                } else {
                    let more = reader.take(body_length - slice_length);
                    (MessageBody::Reader(Box::new(slice.chain(more))), slice.len())
                }
            }
            _ => (MessageBody::None, 0)
        }
    }

    fn format(&self, format: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageBody::Reader(_) => {
                format.write_str("streaming")
            },
            MessageBody::Slice(slice) => {
                if let Ok(result) = str::from_utf8(slice) {
                    format.write_str(result)
                } else {
                    Ok(())
                }
            },
            _ => Ok(()),
        }
    }
}

impl<'a> Drop for MessageBody<'a> {
    fn drop(&mut self) {
        if let MessageBody::Reader(ref mut reader) = *self {
            copy(reader, &mut sink()).expect("should be able to copy");
        }
    }
}

impl<'a> PartialEq for MessageBody<'a> {
    fn eq(&self, other: &MessageBody) -> bool {
        match (self, other) {
            (&MessageBody::None, &MessageBody::None) | (&MessageBody::Reader(_), &MessageBody::Reader(_)) => true,
            (&MessageBody::Slice(slice_a), &MessageBody::Slice(slice_b)) => slice_a == slice_b,
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
            MessageBody::Slice(slice) => {
                writer.write(slice)
            },
            _ => Ok(0),
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
pub struct ChunkExtensions<'a> (pub Vec<(&'a str, Option<Cow<'a, str>>)>);

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
    Last(ChunkExtensions<'a>, Headers<'a>),
}

impl<'a> Chunk<'a> {
    pub fn read(slice: &[u8]) -> Result<(Chunk, usize)> {
        use grammar::*;
        use parser::result;

        let ((size, extensions), remainder) = result(chunk_head(slice))?;
        if size > 0 {
            let s = size as usize;
            let consumed = (slice.len() - remainder.len()) + s + 2;
            Ok((Chunk::Slice(extensions, &remainder[..s]), consumed))
        } else {
            let (trailers, remainder) = result(headers(remainder))?;
            let consumed = (slice.len() - remainder.len()) + 2;
            Ok((Chunk::Last(extensions, trailers), consumed))
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct ChunkedBody<'a> {
    chunks: Vec<Chunk<'a>>,
}

impl<'a> ChunkedBody<'a> {
    pub fn new(mut chunks: Vec<Chunk<'a>>, last: ChunkExtensions<'a>, trailers: Headers<'a>) -> ChunkedBody<'a> {
        chunks.push(Chunk::Last(last, trailers));
        ChunkedBody {
            chunks: chunks,
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct TransferParameter<'a> {
    name: &'a str,
    value: Option<Cow<'a, str>>,
}

impl<'a> TransferParameter<'a> {
    pub fn new<V>(name: &'a str, value: Option<V>) -> TransferParameter<'a>
        where V: Into<Cow<'a, str>> {
        TransferParameter { name: name, value: value.map(|v| v.into()) }
    }
}


#[derive(PartialEq, Debug)]
pub enum TransferCoding<'a> {
    Chunked,
    Compress,
    Deflate,
    Gzip,
    Extension(&'a str, Vec<TransferParameter<'a>>),
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
        assert_eq!(format!("{}", RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } }), "GET /where?q=now HTTP/1.1\r\n");
    }

    #[test]
    fn status_line_display() {
        assert_eq!(format!("{}", StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" }), "HTTP/1.1 200 OK\r\n");
    }

    #[test]
    fn start_line_display() {
        assert_eq!(format!("{}", StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } })), "GET /where?q=now HTTP/1.1\r\n");
    }

    #[test]
    fn headers_display() {
        assert_eq!(format!("{}", Headers(vec!(Header::new("Content-Type", "plain/text"), Header::new("Content-Length", "3")))), "Content-Type: plain/text\r\nContent-Length: 3\r\n");
    }

    #[test]
    fn message_body_display() {
        assert_eq!(format!("{}", MessageBody::Slice(&b"abc"[..])), "abc");
        assert_eq!(format!("{}", MessageBody::None), "");
    }

    #[test]
    fn http_message_display() {
        assert_eq!(format!("{}", HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" }),
            headers: Headers(vec!(Header::new("Content-Type", "plain/text"), Header::new("Content-Length", "3"))),
            body: MessageBody::Slice(&b"abc"[..]),
        }), "HTTP/1.1 200 OK\r\nContent-Type: plain/text\r\nContent-Length: 3\r\n\r\nabc");
    }

    #[test]
    fn can_parse_transfer_encoding() {
        {
            let headers = Headers(vec!(Header::new("Transfer-Encoding", "gzip, chunked"), Header::new("Content-Type", "plain/text")));
            assert_eq!(headers.transfer_encoding(), vec![TransferCoding::Gzip, TransferCoding::Chunked])
        }

        {
            let headers = Headers(vec!(Header::new("Transfer-Encoding", "gzip"), Header::new("Content-Type", "plain/text"), Header::new("Transfer-Encoding", "chunked")));
            assert_eq!(headers.transfer_encoding(), vec![TransferCoding::Gzip, TransferCoding::Chunked])
        }
    }
}