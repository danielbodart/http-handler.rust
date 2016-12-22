use std::ascii::AsciiExt;
use ast::*;

impl <'a> Headers<'a>{
    pub fn get(&'a self, name:&str) -> Option<&'a str> {
        (&self.0).into_iter()
            .find(|&&(key,_)| name.eq_ignore_ascii_case(key))
            .map(|&(_, ref value)| value.as_str())
    }

    pub fn content_length(&self) -> u64 {
        self.get("Content-Length").
            and_then(|value| value.parse::<u64>().ok()).
            unwrap_or(0u64)
    }
}