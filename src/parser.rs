extern crate nom;

use std::io::{Error, ErrorKind, Result};
use std::fmt;
use nom::IResult;

#[macro_export] macro_rules! char_predicate {
    ($i:expr, $c: expr) => {
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
    };
}

pub fn result<I, O>(result: IResult<I, O>) -> Result<(O, I)> where I: fmt::Debug {
    match result {
        IResult::Done(remainder, output) => {
            Ok((output, remainder))
        },
        IResult::Incomplete(needed) => {
            Err(Error::new(ErrorKind::Other, format!("Needs more data: {:?}", needed)))
        },
        IResult::Error(err) => {
            Err(Error::new(ErrorKind::Other, format!("{}", err)))
        },
    }
}