#[macro_use] extern crate nom;

use nom::{digit, is_digit, is_alphabetic};
use std::str;

// HTTP-name     = %x48.54.54.50 ; "HTTP", case-sensitive
named!(http_name, tag!("HTTP"));

#[derive(PartialEq, Debug)]
pub struct HttpVersion {
    major: u8,
    minor: u8,
}

#[macro_export]
macro_rules! filter (
  ($i:expr, $c: expr) => (
    {
      if $i.is_empty() {
        nom::IResult::Incomplete(nom::Needed::Size(1))
      } else {
        if $c($i[0]) {
          nom::IResult::Done(&$i[1..], &$i[0..1])
        } else {
          nom::IResult::Error(error_position!(nom::ErrorKind::Char, $i))
        }
      }
    }
  );
);

fn as_digit(slice: &[u8]) -> u8 {
    slice[0] - 48
}

macro_rules! or {
    ( $( $predicate:expr ),* ) => {
        Box::new(move |chr| {
            $( $predicate(chr) || )* false
        })
    };
}

macro_rules! and {
    ( $( $predicate:expr ),* ) => {
        Box::new(move |chr| {
            $( $predicate(chr) && )* true
        })
    };
}

pub fn among<'a>(characters:&'a str) -> Box<Fn(u8) -> bool + 'a> {
    Box::new(move |chr| {
        characters.chars().any(|it| it == chr as char)
    })
}

// HTTP-version  = HTTP-name "/" DIGIT "." DIGIT
named!(http_version<HttpVersion>, do_parse!(
    http_name >> tag!("/") >> major: digit >> tag!(".") >> minor: digit >>
    (HttpVersion { major: as_digit(major), minor: as_digit(minor)})
  ));

named!(space, tag!(" "));
named!(crlf, tag!("\r\n"));

// TODO: Do we need to be stricter?
named!(request_target, is_not_s!(" "));


// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." / "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
named!(tchar, filter!(or!(among("!#$%&'*+-.^_`|~"), is_digit, is_alphabetic)));

////token = 1*tchar
//named!(token, many1!(tchar));

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
    use super::*;
    use nom::IResult;

    #[test]
    fn http_name() {
        assert_eq!(super::http_name(&b"HTTP"[..]), IResult::Done(&b""[..], &b"HTTP"[..]));
    }

    #[test]
    fn http_version() {
        assert_eq!(super::http_version(&b"HTTP/1.2"[..]), IResult::Done(&b""[..], HttpVersion { major: 1, minor: 2 }));
    }

    #[test]
    fn tchar() {
        assert_eq!(super::tchar(&b"abc"[..]), IResult::Done(&b"bc"[..], &b"a"[..]));
    }

    #[test]
    fn request_line() {
        //        assert_eq!(super::request_line(&b"GET /a/test HTTP/1.1\r\n"[..]), IResult::Done(&b""[..], (&b"GET"[..], &b"/a/test"[..], (&b"1"[..], &b"1"[..]))));
    }
}
