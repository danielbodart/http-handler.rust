extern crate nom;

use nom::{digit, is_digit, is_alphabetic};
use misc::*;
use ast::*;
use predicates::*;

// HTTP-name     = %x48.54.54.50 ; "HTTP", case-sensitive
named!(http_name, tag!("HTTP"));

// HTTP-version  = HTTP-name "/" DIGIT "." DIGIT
named!(http_version<HttpVersion>, do_parse!(
    http_name >> tag!("/") >> major: digit >> tag!(".") >> minor: digit >>
    (HttpVersion { major: as_digit(major), minor: as_digit(minor)})
  ));

named!(space, tag!(" "));
named!(crlf, tag!("\r\n"));

// TODO: full impl
named!(request_target, is_not_s!(" "));


// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
named!(tchar, filter!(or!(among("!#$%&'*+-.^_`|~"), is_digit, is_alphabetic)));

////token = 1*tchar
named!(token, map!(many1!(tchar), join_vec));

//method = token
//named!(method, call!(token));

//request-line   = method SP request-target SP HTTP-version CRLF
//named!(request_line, do_parse!(
//    m: token >> space >> uri: request_target >> space >> version: http_version >> crlf
//    (m, uri, version)
//  ));

/*
start-line     = request-line / status-line


 HTTP-message   = start-line
                      *( header-field CRLF )
                      CRLF
                      [ message-body ]
 */


#[cfg(test)]
mod tests {
    use ast::HttpVersion;
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
        assert_eq!(super::request_target(&b"/where?q=now"[..]), Done(&b""[..], &b"/where?q=now"[..]));
        assert_eq!(super::request_target(&b"http://www.example.org/pub/WWW/TheProject.html"[..]), Done(&b""[..], &b"http://www.example.org/pub/WWW/TheProject.html"[..]));
        assert_eq!(super::request_target(&b"www.example.com:80"[..]), Done(&b""[..], &b"www.example.com:80"[..]));
        assert_eq!(super::request_target(&b"*"[..]), Done(&b""[..], &b"*"[..]));
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
    fn request_line() {
        //        assert_eq!(super::request_line(&b"GET /a/test HTTP/1.1\r\n"[..]), Done(&b""[..], (&b"GET"[..], &b"/a/test"[..], (&b"1"[..], &b"1"[..]))));
    }
}