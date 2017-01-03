extern crate nom;

use nom::{is_digit, is_hex_digit, is_alphabetic, IResult};
use std::{str};

use misc::*;
use ast::*;
use predicates::*;

// HTTP-name     = %x48.54.54.50 ; "HTTP", case-sensitive
named!(http_name, tag!("HTTP"));

//  DIGIT          =  %x30-39 ; 0-9
named!(digit, char_predicate!(is_digit));

// HEXDIG (hexadecimal 0-9/A-F/a-f)
named!(hex_digit, char_predicate!(is_hex_digit));

// HTTP-version  = HTTP-name "/" DIGIT "." DIGIT
named!(http_version <HttpVersion>, do_parse!(
    http_name >> tag!("/") >> major: digit >> tag!(".") >> minor: digit >>
    (HttpVersion { major: asci_digit(major), minor: asci_digit(minor)})
  ));

// SP             =  %x20
named!(space, tag!(" "));
// CRLF           =  CR LF ; Internet standard newline
named!(crlf, tag!("\r\n"));
// HTAB           =  %x09 ; horizontal tab
named!(htab, tag!("\t"));
// VCHAR          =  %x21-7E ; visible (printing) characters
named!(vchar, char_predicate!(range(0x21,0x7E)));
// obs-text       = %x80-FF ; obsolete text
named!(obs_text, char_predicate!(range(0x80,0xFF)));
// OWS            = *( SP / HTAB ) ; optional whitespace
named!(ows, map_res!(many0!(alt!(space | htab)), join_vec));
// RWS            = 1*( SP / HTAB ) ; required whitespace
named!(rws, map_res!(many1!(alt!(space | htab)), join_vec));
// BWS            = OWS ; "bad" whitespace

// DQUOTE         =  %x22 ; " (Double Quote)
named!(double_quote, tag!("\""));

// qdtext         = HTAB / SP / %x21 / %x23-5B / %x5D-7E / obs-text
named!(quoted_text, alt!(htab | space | char_predicate!(or!(ch(0x21), range(0x23,0x5B), range(0x5D,0x7E))) | obs_text ));

// quoted-pair    = "\" ( HTAB / SP / VCHAR / obs-text )
named!(quoted_pair, preceded!(char!('\\'), alt!(htab | space | vchar | obs_text )));

// quoted-string  = DQUOTE *( qdtext / quoted-pair ) DQUOTE
named!(quoted_string <String>, delimited!(double_quote, map_res!(many0!(alt!(quoted_text | quoted_pair)), to_string), double_quote));

// TODO: full impl
named!(request_target <&str>, map_res!(is_not!(" "), str::from_utf8));


// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
named!(tchar, char_predicate!(or!(among("!#$%&'*+-.^_`|~"), is_digit, is_alphabetic)));

////token = 1*tchar
named!(token, map_res!(many1!(tchar), join_vec));

//method = token
named!(method <&str>, map_res!(token, str::from_utf8));

//request-line   = method SP request-target SP HTTP-version CRLF
named!(request_line <RequestLine>, do_parse!(
    method: method >> space >> request_target: request_target >> space >> version: http_version >> crlf >>
    (RequestLine { method: method, request_target: request_target, version: version })
  ));

//status-code    = 3DIGIT
named!(status_code <u16>, map_res!(map_res!(map_res!(many_m_n!(3,3, digit), join_vec), str::from_utf8), parse_u16));

//reason-phrase  = *( HTAB / SP / VCHAR / obs-text )
named!(reason_phrase <&str>, map_res!(map_res!(many0!(alt!(htab | space | vchar | obs_text)), join_vec), str::from_utf8));

// status-line = HTTP-version SP status-code SP reason-phrase CRLF
named!(status_line <StatusLine>, do_parse!(
    version: http_version >> space >> status: status_code >> space >> reason_phrase:reason_phrase >> crlf >>
    (StatusLine { version: version, code: status, description: reason_phrase })
  ));


// start-line     = request-line / status-line
named!(start_line <StartLine>, alt!(map!(request_line, StartLine::RequestLine) | map!(status_line, StartLine::StatusLine)));

// field-name     = token
named!(field_name <&str>, map_res!(token, str::from_utf8));

// field-vchar    = VCHAR / obs-text
named!(field_vchar, alt!(vchar | obs_text));

named!(spaces, map_res!(many1!(alt!(space | htab)), join_vec));

// field-content  = field-vchar [ 1*( SP / HTAB ) field-vchar ]
named!(field_content, do_parse!(
    chr:field_vchar >>
    optional: opt!(complete!(map_res!(pair!( spaces, field_vchar), join_pair))) >>
    (match optional {
        Some(other) => join_slice(chr, other).unwrap(),
        None => chr,
    })
  ));

// obs-fold       = CRLF 1*( SP / HTAB ) ; obsolete line folding
named!(obs_fold, do_parse!( crlf >> spaces >> (Default::default()) ));

// field-value    = *( field-content / obs-fold )
named!(field_value <String>, map_res!(many0!(alt!(field_content | obs_fold)), to_string));

// header-field   = field-name ":" OWS field-value OWS
named!(header_field <(&str, String)>, do_parse!(
    name:field_name >> tag!(":") >> ows >> value:field_value >> ows >>
    ((name, value))
  ));

pub fn message_body<'a>(slice: &'a [u8], headers: &Headers<'a>) -> IResult<&'a [u8], MessageBody<'a>> {
    let length = headers.content_length();
    if length == 0 {
        IResult::Done(slice, MessageBody::None)
    } else {
        match take!(slice, length) {
            IResult::Done(rest, body) => IResult::Done(rest, MessageBody::Slice(body)),
            IResult::Error(e) => IResult::Error(e),
            IResult::Incomplete(n) => IResult::Incomplete(n),
        }
    }
}

named!(headers <Headers>, map!(many0!(terminated!(header_field, crlf)), Headers));


// HTTP-message = start-line *( header-field CRLF ) CRLF [ message-body ]
named!(pub http_message <HttpMessage> , do_parse!(
    start_line:start_line >> headers:headers >> crlf >> body:apply!(message_body, &headers) >>
    (HttpMessage { start_line:start_line, headers:headers, body:body})
  ));

// chunk-size     = 1*HEXDIG
named!(chunk_size <u64>, map_res!(map_res!(map_res!(many1!(hex_digit), join_vec), str::from_utf8), parse_hex));

// chunk-ext-name = token
named!(chunk_ext_name <&str>, map_res!(token, str::from_utf8));

// chunk-ext-val  = token / quoted-string
named!(chunk_ext_value <String>, alt!(map_res!(token, to_owned_string) | quoted_string));

//  chunk-ext      = *( BWS  ";" BWS chunk-ext-name [ BWS  "=" BWS chunk-ext-val ] )
named!(chunk_ext <ChunkExtensions>, map!(many0!(do_parse!(
    ows >> char!(';') >> ows >> name:chunk_ext_name >> value:opt!(complete!(preceded!(delimited!(ows, char!('='), ows), chunk_ext_value))) >>
    (name, value)
)), ChunkExtensions));

// chunk-data     = 1*OCTET ; a sequence of chunk-size octets

// chunk          = chunk-size [ chunk-ext ] CRLF chunk-data CRLF
named!(chunk <Chunk>, do_parse!(
    size:chunk_size >> extensions:chunk_ext >> crlf >> data:take!(size) >> crlf >>
    (Chunk::Slice(extensions, data))
));

// last-chunk     = 1*("0") [ chunk-ext ] CRLF
named!(last_chunk <Chunk>, do_parse!(
    many1!(char!('0')) >> extensions:chunk_ext >> crlf >>
    (Chunk::Last(extensions))
));

// trailer-part   = *( header-field CRLF )
// TODO: Alias headers

// chunked-body   = *chunk last-chunk trailer-part CRLF
named!(chunked_body <ChunkedBody>, do_parse!(
    chunks:many0!(chunk) >> last:last_chunk >> trailers:headers >> crlf >>
    (ChunkedBody::new(chunks, last, trailers))
));


#[cfg(test)]
mod tests {
    use ast::*;
    use nom::IResult::Done;

    #[test]
    fn http_name() {
        assert_eq!(super::http_name(&b"HTTP"[..]), Done(&b""[..], &b"HTTP"[..]));
    }

    #[test]
    fn http_version() {
        assert_eq!(super::http_version(&b"HTTP/1.1"[..]), Done(&b""[..], HttpVersion { major: 1, minor: 1 }));
    }

    #[test]
    fn request_target() {
        assert_eq!(super::request_target(&b"/where?q=now"[..]), Done(&b""[..], "/where?q=now"));
        assert_eq!(super::request_target(&b"http://www.example.org/pub/WWW/TheProject.html"[..]), Done(&b""[..], "http://www.example.org/pub/WWW/TheProject.html"));
        assert_eq!(super::request_target(&b"www.example.com:80"[..]), Done(&b""[..], "www.example.com:80"));
        assert_eq!(super::request_target(&b"*"[..]), Done(&b""[..], "*"));
    }

    #[test]
    fn tchar() {
        assert_eq!(super::tchar(&b"abc"[..]), Done(&b"bc"[..], &b"a"[..]));
    }

    #[test]
    fn token() {
        assert_eq!(super::token(&b"abc"[..]), Done(&b""[..], &b"abc"[..]));
    }

    #[test]
    fn method() {
        assert_eq!(super::method(&b"GET"[..]), Done(&b""[..], "GET"));
    }

    #[test]
    fn request_line() {
        assert_eq!(super::request_line(&b"GET /where?q=now HTTP/1.1\r\n"[..]), Done(&b""[..], RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1, } }));
    }

    #[test]
    fn status_code() {
        assert_eq!(super::status_code(&b"200"[..]), Done(&b""[..], 200));
    }

    #[test]
    fn reason_phrase() {
        assert_eq!(super::reason_phrase(&b"OK"[..]), Done(&b""[..], "OK"));
        assert_eq!(super::reason_phrase(&b"Not Found"[..]), Done(&b""[..], "Not Found"));
    }

    #[test]
    fn status_line() {
        assert_eq!(super::status_line(&b"HTTP/1.1 200 OK\r\n"[..]), Done(&b""[..], StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }));
    }

    #[test]
    fn start_line() {
        assert_eq!(super::start_line(&b"GET /where?q=now HTTP/1.1\r\n"[..]), Done(&b""[..], StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1, } })));
        assert_eq!(super::start_line(&b"HTTP/1.1 200 OK\r\n"[..]), Done(&b""[..], StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" })));
    }

    #[test]
    fn field_name() {
        assert_eq!(super::field_name(&b"Content-Type"[..]), Done(&b""[..], "Content-Type"));
    }

    #[test]
    fn field_content() {
        assert_eq!(super::field_content(&b"a  b"[..]), Done(&b""[..], &b"a  b"[..]));
        assert_eq!(super::field_content(&b"a b"[..]), Done(&b""[..], &b"a b"[..]));
        assert_eq!(super::field_content(&b"a"[..]), Done(&b""[..], &b"a"[..]));
    }

    #[test]
    fn field_value() {
        assert_eq!(super::field_value(&b"plain/text"[..]), Done(&b""[..], "plain/text".to_string()));
        assert_eq!(super::field_value(&b"Spaces are allowed in the middle"[..]), Done(&b""[..], "Spaces are allowed in the middle".to_string()));
        assert_eq!(super::field_value(&b"You can al\r\n so wrap onto new lines!"[..]), Done(&b""[..], "You can also wrap onto new lines!".to_string()));
    }

    #[test]
    fn header_field() {
        assert_eq!(super::header_field(&b"Content-Type:plain/text"[..]), Done(&b""[..], ("Content-Type", "plain/text".to_string())));
        assert_eq!(super::header_field(&b"Content-Type: plain/text"[..]), Done(&b""[..], ("Content-Type", "plain/text".to_string())));
        assert_eq!(super::header_field(&b"Content-Type: plain/text "[..]), Done(&b""[..], ("Content-Type", "plain/text".to_string())));
        assert_eq!(super::header_field(&b"Content-Type: plain/\r\n text "[..]), Done(&b""[..], ("Content-Type", "plain/text".to_string())));
    }

    #[test]
    fn http_message() {
        assert_eq!(super::http_message(&b"GET /where?q=now HTTP/1.1\r\nContent-Type:plain/text\r\n\r\n"[..]), Done(&b""[..], HttpMessage {
            start_line: StartLine::RequestLine(RequestLine { method: "GET", request_target: "/where?q=now", version: HttpVersion { major: 1, minor: 1, } }),
            headers: Headers(vec!(("Content-Type", "plain/text".to_string()))),
            body: MessageBody::None,
        }));
        assert_eq!(super::http_message(&b"HTTP/1.1 200 OK\r\nContent-Type:plain/text\r\n\r\n"[..]), Done(&b""[..], HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "plain/text".to_string()))),
            body: MessageBody::None,
        }));
        assert_eq!(super::http_message(&b"HTTP/1.1 200 OK\r\nContent-Type:plain/text\r\nContent-Length:3\r\n\r\nabc"[..]), Done(&b""[..], HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "plain/text".to_string()), ("Content-Length", "3".to_string()))),
            body: MessageBody::Slice(&b"abc"[..]),
        }));
    }

    #[test]
    fn chunk_size() {
        assert_eq!(super::chunk_size(&b"4\r\n"[..]), Done(&b"\r\n"[..], 4));
        assert_eq!(super::chunk_size(&b"E\r\n"[..]), Done(&b"\r\n"[..], 14));
        assert_eq!(super::chunk_size(&b"e\r\n"[..]), Done(&b"\r\n"[..], 14));
    }

    #[test]
    fn quoted_string() {
        assert_eq!(super::quoted_string(&b"\"This is a quoted string\""[..]), Done(&b""[..], "This is a quoted string".to_string()));
        assert_eq!(super::quoted_string(&b"\"This is a \\\"quoted\\\" string\""[..]), Done(&b""[..], "This is a \"quoted\" string".to_string()));
    }

    #[test]
    fn chunk_ext() {
        assert_eq!(super::chunk_ext(&b";foo=bar"[..]), Done(&b""[..], ChunkExtensions(vec!(("foo", Some("bar".to_string()))))));
        assert_eq!(super::chunk_ext(&b";foo"[..]), Done(&b""[..], ChunkExtensions(vec!(("foo", None)))));
        assert_eq!(super::chunk_ext(&b";foo=bar;baz"[..]), Done(&b""[..], ChunkExtensions(vec!(("foo", Some("bar".to_string())), ("baz", None)))));
        assert_eq!(super::chunk_ext(&b" ; foo = bar ; baz"[..]), Done(&b""[..], ChunkExtensions(vec!(("foo", Some("bar".to_string())), ("baz", None)))));
        assert_eq!(super::chunk_ext(&b""[..]), Done(&b""[..], ChunkExtensions(vec!())));
    }

    #[test]
    fn chunk() {
        assert_eq!(super::chunk(&b"4;foo=bar\r\nWiki\r\n"[..]), Done(&b""[..], Chunk::Slice(ChunkExtensions(vec!(("foo", Some("bar".to_string())))), &b"Wiki"[..])));
    }


    #[test]
    fn chunked_body() {
        let chunked_body = ChunkedBody::new(vec!(
            Chunk::Slice(ChunkExtensions(vec!()), &b"Wiki"[..]),
            Chunk::Slice(ChunkExtensions(vec!()), &b"pedia"[..]),
            Chunk::Slice(ChunkExtensions(vec!()), &b" in\r\n\r\nchunks."[..])),
                                 Chunk::Last(ChunkExtensions(vec!())),
                                 Headers(vec!()));
        assert_eq!(super::chunked_body(&b"4\r\nWiki\r\n5\r\npedia\r\nE\r\n in\r\n\r\nchunks.\r\n0\r\n\r\n"[..]),
        Done(&b""[..], chunked_body));
    }
}