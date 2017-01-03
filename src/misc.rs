use std::{slice, num, string, fmt};
use std::error::Error;

#[derive(PartialEq, Debug)]
pub enum SliceError {
    NotAdjacent,
}

impl Error for SliceError {
    fn description(&self) -> &str {
        match *self {
            SliceError::NotAdjacent => "Can not join slices that are not next to each other",
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl<'a> fmt::Display for SliceError {
    fn fmt(&self, format: &mut fmt::Formatter) -> fmt::Result {
        format.write_str(self.description())
    }
}

pub fn join_slice<'a>(slice1: &'a [u8], slice2: &'a [u8]) -> Result<&'a [u8], SliceError> {
    if slice1.is_empty() { return Ok(slice2); }
    if slice2.is_empty() { return Ok(slice1); }
    unsafe {
        if is_adjacent(slice1, slice2) {
            Ok(slice::from_raw_parts(slice1.as_ptr(), slice1.len() + slice2.len()))
        } else {
            Err(SliceError::NotAdjacent)
        }
    }
}

pub fn is_adjacent<'a>(slice1: &'a [u8], slice2: &'a [u8]) -> bool {
    unsafe {
        slice2.as_ptr() == slice1.as_ptr().offset(slice1.len() as isize)
    }
}

pub fn join_vec<'a>(vec: Vec<&'a [u8]>) -> Result<&'a [u8], SliceError> {
    if vec.is_empty() {
        return Ok(Default::default());
    }
    vec.into_iter().fold(Ok(Default::default()), |a, slice2| {
        if let Ok(slice1) = a { return join_slice(slice1, slice2) }
        a
    })
}

pub fn join_pair<'a>(pair: (&'a [u8], &'a [u8])) -> Result<&'a [u8], SliceError> {
    join_slice(pair.0, pair.1)
}

pub fn asci_digit(slice: &[u8]) -> u8 {
    slice[0] - 48
}

pub fn parse_u8(value: &str) -> Result<u8, num::ParseIntError> {
    value.parse::<u8>()
}

pub fn parse_u16(value: &str) -> Result<u16, num::ParseIntError> {
    value.parse::<u16>()
}

pub fn parse_hex(value: &str) -> Result<u64, num::ParseIntError> {
    u64::from_str_radix(value, 16)
}

pub fn to_string(vec:Vec<&[u8]>) -> Result<String, string::FromUtf8Error> {
    String::from_utf8(vec.concat())
}

pub fn to_owned_string(slice:&[u8]) -> Result<String, string::FromUtf8Error> {
    String::from_utf8(slice.to_vec())
}


#[cfg(test)]
mod tests {
    #[test]
    fn join_slice() {
        let bytes = b"HTTP";
        assert_eq!(super::join_slice(&bytes[0..2], &bytes[2..4]), Ok(&bytes[0..4]));
        assert_eq!(super::join_slice(Default::default(), &bytes[2..4]), Ok(&bytes[2..4]));
        assert_eq!(super::join_slice(&bytes[0..2], Default::default()), Ok(&bytes[0..2]));
    }

    #[test]
    fn join_vec() {
        let bytes = b"HTTP";
        let vec = vec![&bytes[0..2], &bytes[2..4]];
        assert_eq!(super::join_vec(vec), Ok(&bytes[0..4]));
    }
}