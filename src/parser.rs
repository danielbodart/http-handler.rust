extern crate nom;

use std::io::{Result};
use std::fmt;
use nom::IResult;
use crate::io::SimpleError;

#[macro_export] macro_rules! char_predicate {
    ($i:expr, $c: expr) => {
        {
            if $i.is_empty() {
                std::result::Result::Err(nom::Err::Incomplete(nom::Needed::Size(1)))
            } else {
                if $c($i[0]) {
                    std::result::Result::Ok((&$i[1..], &$i[0..1]))
                } else {
                    std::result::Result::Err(nom::Err::Error(error_position!($i, nom::error::ErrorKind::Char)))
                }
            }
        }
    };
}

pub fn result<I, O>(result: IResult<I, O>) -> Result<(O, I)> where I: fmt::Debug {
    match result {
        Ok((remainder, output)) => {
            Ok((output, remainder))
        },
        Err(nom::Err::Incomplete(needed)) => {
            Err(SimpleError::debug(needed))
        },
        Err(nom::Err::Error(err)) => {
            Err(SimpleError::debug(err))
        },
        Err(nom::Err::Failure(f)) => {
            Err(SimpleError::debug(f))
        }
    }
}