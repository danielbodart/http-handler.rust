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

pub fn among<'a>(characters: &'a str) -> Box<dyn Fn(u8) -> bool + 'a> {
    Box::new(move |chr| {
        characters.chars().any(|it| it == chr as char)
    })
}

pub fn range(start:u8, end:u8) -> Box<dyn Fn(u8) -> bool> {
    Box::new(move |chr| {
        chr >= start && chr <= end
    })
}

pub fn ch(value:u8) -> Box<dyn Fn(u8) -> bool> {
    Box::new(move |chr| {
        chr == value
    })
}