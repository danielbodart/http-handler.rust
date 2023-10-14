
#[macro_export]
macro_rules! or {
    ( $( $predicate:expr ),* ) => {
        move |chr| {
            $( $predicate(chr) || )* false
        }
    };
}

#[macro_export] macro_rules! and {
    ( $( $predicate:expr ),* ) => {
        move |chr| {
            $( $predicate(chr) && )* true
        }
    };
}

pub fn among<'a>(characters: &'a str) -> impl Fn(u8) -> bool + 'a {
    move |chr| characters.chars().any(|it| it == chr as char)
}

pub fn range(start: u8, end: u8) -> impl Fn(u8) -> bool {
    move |chr| chr >= start && chr <= end
}

pub fn ch(value: u8) -> impl Fn(u8) -> bool {
    move |chr| chr == value
}

pub fn not<F: Fn(u8) -> bool>(predicate: F) -> impl Fn(u8) -> bool {
    move |chr| !predicate(chr)
}