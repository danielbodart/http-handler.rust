use std::{slice, num};

pub fn join_slice<'a>(slice1: &'a [u8], slice2: &'a [u8]) -> &'a [u8] {
    unsafe {
        assert_eq!(slice2.as_ptr(), slice1.as_ptr().offset(slice1.len() as isize));
        slice::from_raw_parts(slice1.as_ptr(), slice1.len() + slice2.len())
    }
}

pub fn join_vec<'a>(vec: Vec<&'a [u8]>) -> &'a [u8] {
    let mut it = vec.into_iter();
    it.next().map(|first| it.fold(first, join_slice)).unwrap()
}

pub fn join_pair<'a>(pair: (&'a [u8], &'a [u8])) -> &'a [u8] {
    join_slice(pair.0, pair.1)
}

pub fn asci_digit(slice: &[u8]) -> u8 {
    slice[0] - 48
}

pub fn parse_u8(value: &str) -> Result<u8, num::ParseIntError> {
    value.parse::<u8>()
}

#[cfg(test)]
mod tests {
    #[test]
    fn join_slice() {
        let bytes = b"HTTP";
        assert_eq!(super::join_slice(&bytes[0..2], &bytes[2..4]), &bytes[0..4]);
    }

    #[test]
    fn join_vec() {
        let bytes = b"HTTP";
        let vec = vec![&bytes[0..2], &bytes[2..4]];
        assert_eq!(super::join_vec(vec), &bytes[0..4]);
    }
}