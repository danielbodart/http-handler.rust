extern crate nom;

use std::borrow::Cow;
use std::str;

use nom::character::{complete, is_alphabetic, is_digit, is_hex_digit};
use nom::IResult;
use nom::multi::{separated_nonempty_list, many0};
use nom::sequence::delimited;

use crate::ast::*;
use crate::misc::*;
use crate::predicates::*;

// trailer-part   = *( header-field CRLF )
pub use self::headers as trailer_part;
// BWS            = OWS ; "bad" whitespace
pub use self::ows as bws;
//method = token
pub use self::token as method;
// field-name     = token
pub use self::token as field_name;
// chunk-ext-name = token
pub use self::token as chunk_ext_name;
use nom::bytes::complete::take;
use nom::error::ErrorKind;

// HTTP-name     = %x48.54.54.50 ; "HTTP", case-sensitive
named!(pub http_name, tag!("HTTP"));

//  DIGIT          =  %x30-39 ; 0-9
named!(pub digit, char_predicate!(is_digit));

// HEXDIG (hexadecimal 0-9/A-F/a-f)
named!(pub hex_digit, char_predicate!(is_hex_digit));

// HTTP-version  = HTTP-name "/" DIGIT "." DIGIT
named!(pub http_version <HttpVersion>, do_parse!(
    http_name >> tag!("/") >> major: digit >> tag!(".") >> minor: digit >>
    (HttpVersion { major: asci_digit(major), minor: asci_digit(minor)})
  ));

// SP             =  %x20
named!(pub space, tag!(" "));
// CRLF           =  CR LF ; Internet standard newline
named!(pub crlf, tag!("\r\n"));
// HTAB           =  %x09 ; horizontal tab
named!(pub htab, tag!("\t"));
// VCHAR          =  %x21-7E ; visible (printing) characters
named!(pub vchar, char_predicate!(range(0x21,0x7E)));
// obs-text       = %x80-FF ; obsolete text
named!(pub obs_text, char_predicate!(range(0x80,0xFF)));
// OWS            = *( SP / HTAB ) ; optional whitespace
named!(pub ows, map_res!(many0!(complete!(alt!(space | htab))), join_vec));
// RWS            = 1*( SP / HTAB ) ; required whitespace
named!(pub rws, map_res!(many1!(complete!(alt!(space | htab))), join_vec));
// DQUOTE         =  %x22 ; " (Double Quote)
named!(pub double_quote, tag!("\""));

// qdtext         = HTAB / SP / %x21 / %x23-5B / %x5D-7E / obs-text
named!(pub quoted_text, alt!(htab | space | char_predicate!(or!(ch(0x21), range(0x23,0x5B), range(0x5D,0x7E))) | obs_text ));

// quoted-pair    = "\" ( HTAB / SP / VCHAR / obs-text )
named!(pub quoted_pair, preceded!(char!('\\'), alt!(htab | space | vchar | obs_text )));

// quoted-string  = DQUOTE *( qdtext / quoted-pair ) DQUOTE
named!(pub quoted_string <Cow<str>>, delimited!(double_quote, map_res!(many0!(complete!(alt!(quoted_text | quoted_pair))), to_cow_str), double_quote));

// TODO: full impl
named!(pub request_target <&str>, map_res!(is_not!(" "), str::from_utf8));


// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
named!(pub tchar, char_predicate!(or!(among("!#$%&'*+-.^_`|~"), is_digit, is_alphabetic)));

////token = 1*tchar
named!(pub token <&str>, map_res!(map_res!(many1!(complete!(tchar)), join_vec), str::from_utf8));

//request-line   = method SP request-target SP HTTP-version CRLF
named!(pub request_line <RequestLine>, do_parse!(
    method: method >> space >> request_target: request_target >> space >> version: http_version >> crlf >>
    (RequestLine { method: method, request_target: request_target, version: version })
  ));

//status-code    = 3DIGIT
named!(pub status_code <u16>, map_res!(map_res!(map_res!(many_m_n!(3,3, complete!(digit)), join_vec), str::from_utf8), parse_u16));

//reason-phrase  = *( HTAB / SP / VCHAR / obs-text )
named!(pub reason_phrase <&str>, map_res!(map_res!(many0!(complete!(alt!(htab | space | vchar | obs_text))), join_vec), str::from_utf8));

// status-line = HTTP-version SP status-code SP reason-phrase CRLF
named!(pub status_line <StatusLine>, do_parse!(
    version: http_version >> space >> status: status_code >> space >> reason_phrase:reason_phrase >> crlf >>
    (StatusLine { version: version, code: status, description: reason_phrase })
  ));


// start-line     = request-line / status-line
named!(pub start_line <StartLine>, alt!(map!(request_line, StartLine::RequestLine) | map!(status_line, StartLine::StatusLine)));

// field-vchar    = VCHAR / obs-text
named!(pub field_vchar, alt!(vchar | obs_text));

named!(pub spaces, map_res!(many1!(complete!(alt!(space | htab))), join_vec));

// field-content  = field-vchar [ 1*( SP / HTAB ) field-vchar ]
named!(pub field_content, do_parse!(
    chr:field_vchar >>
    optional: opt!(complete!(map_res!(pair!( spaces, field_vchar), join_pair))) >>
    (match optional {
        Some(other) => join_slice(chr, other).unwrap(),
        None => chr,
    })
  ));

// obs-fold       = CRLF 1*( SP / HTAB ) ; obsolete line folding
named!(pub obs_fold, do_parse!( crlf >> spaces >> (Default::default()) ));

// field-value    = *( field-content / obs-fold )
named!(pub field_value <Cow<str>>, map_res!(many0!(complete!(alt!(field_content | obs_fold))), to_cow_str));

// header-field   = field-name ":" OWS field-value OWS
named!(pub header_field <Header>, do_parse!(
    name:field_name >> tag!(":") >> ows >> value:field_value >> ows >>
    (Header::new(name, value))
  ));

pub fn message_body<'a>(slice: &'a [u8], headers: &Headers<'a>) -> IResult<&'a [u8], MessageBody<'a>> {
    match headers.content_length() {
        Some(length) if length > 0 => {
            match take!(slice, length) {
                Ok((rest, body)) => Ok((rest, MessageBody::Slice(body))),
                Err(nom::Err::Error(c)) => Err(nom::Err::Error(c)),
                Err(nom::Err::Incomplete(n)) => Err(nom::Err::Incomplete(n)),
                Err(nom::Err::Failure(c)) => Err(nom::Err::Failure(c)),
            }
        }
        _ => IResult::Ok((slice, MessageBody::None))
    }
}

named!(pub headers <Headers>, map!(many0!(complete!(terminated!(header_field, crlf))), Headers));

named!(pub message_head <MessageHead> , do_parse!(
    start_line:start_line >> headers:headers >> crlf >>
    (MessageHead { start_line:start_line, headers:headers})
  ));

macro_rules! apply (
  ($i:expr, $fun:expr, $($args:expr),* ) => ( $fun( $i, $($args),* ) );
);

// HTTP-message = start-line *( header-field CRLF ) CRLF [ message-body ]
named!(pub http_message <HttpMessage> , do_parse!(
    head:message_head >> body:apply!(message_body, &head.headers) >>
    (HttpMessage { start_line:head.start_line, headers:head.headers, body:body})
  ));

// chunk-size     = 1*HEXDIG
named!(pub chunk_size <u64>, map_res!(map_res!(map_res!(many1!(complete!(hex_digit)), join_vec), str::from_utf8), parse_hex));

// chunk-ext-val  = token / quoted-string
named!(pub chunk_ext_value <Cow<str>>, alt!(map!(token, Cow::from) | quoted_string));

//  chunk-ext      = *( BWS  ";" BWS chunk-ext-name [ BWS  "=" BWS chunk-ext-val ] )
named!(pub chunk_ext <ChunkExtensions>, map!(many0!(complete!(do_parse!(
    bws >> char!(';') >> bws >> name:chunk_ext_name >> value:opt!(complete!(preceded!(delimited!(bws, char!('='), bws), chunk_ext_value))) >>
    (name, value)
))), ChunkExtensions));

// chunk-data     = 1*OCTET ; a sequence of chunk-size octets
// chunk          = chunk-size [ chunk-ext ] CRLF chunk-data CRLF
pub fn chunk(i: &[u8]) -> nom::IResult<&[u8], Chunk, (&[u8], nom::error::ErrorKind)> {
    let (i, size) = chunk_size(i)?;
    if size == 0 { return Err(nom::Err::Error((i, ErrorKind::Complete))) };
    let (i, extensions) = chunk_ext(i)?;
    let (i, _) = crlf(i)?;
    let (i, data) = take(size)(i)?;
    let (i, _) = crlf(i)?;
    Ok((i, Chunk::Slice(extensions, data)))
}

named!(pub chunk_head <(u64, ChunkExtensions)>, do_parse!(
    size:chunk_size >> extensions:chunk_ext >> crlf >>
    (size, extensions)
));

// last-chunk     = 1*("0") [ chunk-ext ] CRLF
named!(pub last_chunk <ChunkExtensions>, do_parse!(
    many1!(complete!(char!('0'))) >> extensions:chunk_ext >> crlf >>
    (extensions)
));

// chunked-body   = *chunk last-chunk trailer-part CRLF
pub fn chunked_body(i: &[u8]) -> nom::IResult<&[u8], ChunkedBody, (&[u8], nom::error::ErrorKind)> {
    let (i, chunks) = many0(chunk)(i)?;
    let (i, last) = last_chunk(i)?;
    let (i, trailers) = trailer_part(i)?;
    let (i, _) = crlf(i)?;
    Ok((i, ChunkedBody::new(chunks, last, trailers)))
}

// transfer-parameter = token / token BWS "=" BWS ( token / quoted-string )
named!(pub transfer_parameter <TransferParameter>, do_parse!(
    name:token >> bws >> char!('=') >> bws >> value:opt!(complete!(alt!(map!(token, Cow::from) | quoted_string))) >>
    (TransferParameter::new(name, value))
));

// transfer-extension = token *( OWS ";" OWS transfer-parameter )
named!(pub transfer_extension <TransferCoding>, do_parse!(
    name:token >> params:many0!(complete!(do_parse!(ows >> char!(';') >> ows >> param: transfer_parameter >> (param)))) >>
    (TransferCoding::Extension(name, params))
));

// transfer-coding    = "chunked" / "compress" / "deflate" / "gzip" / transfer-extension
named!(pub transfer_coding <TransferCoding>, alt!(
    value!(TransferCoding::Chunked, tag!("chunked")) |
    value!(TransferCoding::Compress, tag!("compress")) |
    value!(TransferCoding::Deflate, tag!("deflate")) |
    value!(TransferCoding::Gzip, tag!("gzip")) |
    transfer_extension
));

//  Transfer-Encoding = 1#transfer-coding
//  #rule: 1#element => element *( OWS "," OWS element )
pub fn transfer_encoding(i: &[u8]) -> nom::IResult<&[u8], Vec<TransferCoding>, (&[u8], nom::error::ErrorKind)> {
    separated_nonempty_list(delimited(ows, complete::char(','), ows), transfer_coding)(i)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::ast::*;

    #[test]
    fn http_name() {
        assert_eq!(super::http_name(&b"HTTP"[..]), Ok((&b""[..], &b"HTTP"[..])));
    }

    #[test]
    fn http_version() {
        assert_eq!(super::http_version(&b"HTTP/1.1"[..]), Ok((&b""[..], HttpVersion { major: 1, minor: 1 })));
    }

    #[test]
    fn request_target() {
        assert_eq!(super::request_target(&b"/where?q=now "[..]), Ok((&b" "[..], "/where?q=now")));
        assert_eq!(super::request_target(&b"http://www.example.org/pub/WWW/TheProject.html "[..]), Ok((&b" "[..], "http://www.example.org/pub/WWW/TheProject.html")));
        assert_eq!(super::request_target(&b"www.example.com:80 "[..]), Ok((&b" "[..], "www.example.com:80")));
        assert_eq!(super::request_target(&b"* "[..]), Ok((&b" "[..], "*")));
    }

    #[test]
    fn tchar() {
        assert_eq!(super::tchar(&b"abc"[..]), Ok((&b"bc"[..], &b"a"[..])));
    }

    #[test]
    #[test]
    fn token() {
        assert_eq!(super::token(&b"abc"[..]), Ok((&b""[..], "abc")));
    }

    #[test]
    fn method() {
        assert_eq!(super::method(&b"GET"[..]), Ok((&b""[..], "GET")));
    }

    #[test]
    fn request_line() {
        assert_eq!(super::request_line(&b"GET /where?q=now HTTP/1.1\r\n"[..]), Ok((&b""[..], RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } })));
    }

    #[test]
    fn status_code() {
        assert_eq!(super::status_code(&b"200"[..]), Ok((&b""[..], 200)));
    }

    #[test]
    fn reason_phrase() {
        assert_eq!(super::reason_phrase(&b"OK"[..]), Ok((&b""[..], "OK")));
        assert_eq!(super::reason_phrase(&b"Not Found"[..]), Ok((&b""[..], "Not Found")));
    }

    #[test]
    fn status_line() {
        assert_eq!(super::status_line(&b"HTTP/1.1 200 OK\r\n"[..]), Ok((&b""[..], StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" })));
    }

    #[test]
    fn start_line() {
        assert_eq!(super::start_line(&b"GET /where?q=now HTTP/1.1\r\n"[..]), Ok((&b""[..], StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } }))));
        assert_eq!(super::start_line(&b"HTTP/1.1 200 OK\r\n"[..]), Ok((&b""[..], StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" }))));
    }

    #[test]
    fn field_name() {
        assert_eq!(super::field_name(&b"Content-Type"[..]), Ok((&b""[..], "Content-Type")));
    }

    #[test]
    fn field_content() {
        assert_eq!(super::field_content(&b"a  b"[..]), Ok((&b""[..], &b"a  b"[..])));
        assert_eq!(super::field_content(&b"a b"[..]), Ok((&b""[..], &b"a b"[..])));
        assert_eq!(super::field_content(&b"a"[..]), Ok((&b""[..], &b"a"[..])));
    }

    #[test]
    fn field_value() {
        assert_eq!(super::field_value(&b"plain/text"[..]), Ok((&b""[..], Cow::from("plain/text"))));
        assert_eq!(super::field_value(&b"Spaces are allowed in the middle"[..]), Ok((&b""[..], Cow::from("Spaces are allowed in the middle"))));
        assert_eq!(super::field_value(&b"You can al\r\n so wrap onto new lines!"[..]), Ok((&b""[..], Cow::from("You can also wrap onto new lines!"))));
    }

    #[test]
    fn header_field() {
        assert_eq!(super::header_field(&b"Content-Type:plain/text"[..]), Ok((&b""[..], Header::new("Content-Type", "plain/text"))));
        assert_eq!(super::header_field(&b"Content-Type: plain/text"[..]), Ok((&b""[..], Header::new("Content-Type", "plain/text"))));
        assert_eq!(super::header_field(&b"Content-Type: plain/text "[..]), Ok((&b""[..], Header::new("Content-Type", "plain/text"))));
        assert_eq!(super::header_field(&b"Content-Type: plain/\r\n text "[..]), Ok((&b""[..], Header::new("Content-Type", "plain/text"))));
    }

    #[test]
    fn http_message() {
        assert_eq!(super::http_message(&b"GET /where?q=now HTTP/1.1\r\nContent-Type:plain/text\r\n\r\n"[..]), Ok((&b""[..], HttpMessage {
            start_line: StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } }),
            headers: Headers(vec!(Header::new("Content-Type", "plain/text"))),
            body: MessageBody::None,
        })));
        assert_eq!(super::http_message(&b"HTTP/1.1 200 OK\r\nContent-Type:plain/text\r\n\r\n"[..]), Ok((&b""[..], HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" }),
            headers: Headers(vec!(Header::new("Content-Type", "plain/text"))),
            body: MessageBody::None,
        })));
        assert_eq!(super::http_message(&b"HTTP/1.1 200 OK\r\nContent-Type:plain/text\r\nContent-Length:3\r\n\r\nabc"[..]), Ok((&b""[..], HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1 }, code: 200, description: "OK" }),
            headers: Headers(vec!(Header::new("Content-Type", "plain/text"), Header::new("Content-Length", "3"))),
            body: MessageBody::Slice(&b"abc"[..]),
        })));
    }

    #[test]
    fn chunk_size() {
        assert_eq!(super::chunk_size(&b"4\r\n"[..]), Ok((&b"\r\n"[..], 4)));
        assert_eq!(super::chunk_size(&b"E\r\n"[..]), Ok((&b"\r\n"[..], 14)));
        assert_eq!(super::chunk_size(&b"e\r\n"[..]), Ok((&b"\r\n"[..], 14)));
    }

    #[test]
    fn quoted_string() {
        assert_eq!(super::quoted_string(&b"\"This is a quoted string\""[..]), Ok((&b""[..], Cow::from("This is a quoted string"))));
        assert_eq!(super::quoted_string(&b"\"This is a \\\"quoted\\\" string\""[..]), Ok((&b""[..], Cow::from("This is a \"quoted\" string"))));
    }

    #[test]
    fn chunk_ext() {
        assert_eq!(super::chunk_ext(&b";foo=bar"[..]), Ok((&b""[..], ChunkExtensions(vec!(("foo", Some(Cow::from("bar"))))))));
        assert_eq!(super::chunk_ext(&b";foo"[..]), Ok((&b""[..], ChunkExtensions(vec!(("foo", None))))));
        assert_eq!(super::chunk_ext(&b";foo=bar;baz"[..]), Ok((&b""[..], ChunkExtensions(vec!(("foo", Some(Cow::from("bar"))), ("baz", None))))));
        assert_eq!(super::chunk_ext(&b" ; foo = bar ; baz"[..]), Ok((&b""[..], ChunkExtensions(vec!(("foo", Some(Cow::from("bar"))), ("baz", None))))));
        assert_eq!(super::chunk_ext(&b""[..]), Ok((&b""[..], ChunkExtensions(vec!()))));
    }

    #[test]
    fn chunk() {
        assert_eq!(super::chunk(&b"4;foo=bar\r\nWiki\r\n"[..]), Ok((&b""[..], Chunk::Slice(ChunkExtensions(vec!(("foo", Some(Cow::from("bar"))))), &b"Wiki"[..]))));
    }


    #[test]
    fn chunked_body() {
        let chunked_body = ChunkedBody::new(vec!(
            Chunk::Slice(ChunkExtensions(vec!()), &b"Wiki"[..]),
            Chunk::Slice(ChunkExtensions(vec!()), &b"pedia"[..]),
            Chunk::Slice(ChunkExtensions(vec!()), &b" in\r\n\r\nchunks."[..])),
                                            ChunkExtensions(vec!()), Headers(vec!()));
        assert_eq!(super::chunked_body(&b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\n"[..]),
                   Ok((&b""[..], chunked_body)));
    }

    #[test]
    fn message_head() {
        assert_eq!(super::message_head(&b"POST /where?q=now HTTP/1.1\r\nContent-Type:plain/text\r\nContent-Length:3\r\n\r\nabc"[..]), Ok((&b"abc"[..], MessageHead {
            start_line: StartLine::RequestLine(RequestLine { method: "POST", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1 } }),
            headers: Headers(vec!(Header::new("Content-Type", "plain/text"), Header::new("Content-Length", "3"))),
        })));
    }

    #[test]
    fn transfer_coding() {
        assert_eq!(super::transfer_coding(&b"chunked"[..]), Ok((&b""[..], TransferCoding::Chunked)));
        assert_eq!(super::transfer_coding(&b"compress"[..]), Ok((&b""[..], TransferCoding::Compress)));
        assert_eq!(super::transfer_coding(&b"deflate"[..]), Ok((&b""[..], TransferCoding::Deflate)));
        assert_eq!(super::transfer_coding(&b"gzip"[..]), Ok((&b""[..], TransferCoding::Gzip)));
        assert_eq!(super::transfer_coding(&b"cat ; foo=bar"[..]), Ok((&b""[..], TransferCoding::Extension("cat", vec![TransferParameter::new("foo", Some("bar"))]))));
    }

    #[test]
    fn transfer_encoding() {
        assert_eq!(super::transfer_encoding(&b"gzip, chunked"[..]), Ok((&b""[..], vec![TransferCoding::Gzip, TransferCoding::Chunked])));
        assert_eq!(super::transfer_encoding(&b"chunked"[..]), Ok((&b""[..], vec![TransferCoding::Chunked])));
    }
}