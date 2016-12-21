extern crate nom;

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

