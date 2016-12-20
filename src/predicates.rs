extern crate nom;

#[macro_export] macro_rules! char_predicate (
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

#[macro_export] macro_rules! or {
    ( $( $predicate:expr ),* ) => {
        Box::new(move |chr| {
            $( $predicate(chr) || )* false
        })
    };
}

#[macro_export] macro_rules! and {
    ( $( $predicate:expr ),* ) => {
        Box::new(move |chr| {
            $( $predicate(chr) && )* true
        })
    };
}

pub fn among<'a>(characters: &'a str) -> Box<Fn(u8) -> bool + 'a> {
    Box::new(move |chr| {
        characters.chars().any(|it| it == chr as char)
    })
}
