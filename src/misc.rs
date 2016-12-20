extern crate nom;

use std::slice;

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

pub fn join<'a>(slice1: &'a [u8], slice2: &'a [u8]) -> &'a [u8] {
    unsafe {
        assert_eq!(slice2.as_ptr(), slice1.as_ptr().offset(slice1.len() as isize));
        slice::from_raw_parts(slice1.as_ptr(), slice1.len() + slice2.len())
    }
}

pub fn as_digit(slice: &[u8]) -> u8 {
    slice[0] - 48
}

#[cfg(test)]
mod tests {
    #[test]
    fn join() {
        let array = b"HTTP";
        assert_eq!(super::join(&array[0..2], &array[2..4]), &array[0..4]);
    }
}
