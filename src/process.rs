use std::{env, process};
use std::marker::Sized;
use std::error::Error;
use std::iter::FromIterator;
use std::collections::HashMap;

pub trait Process<E> where Self: Sized, E: Error {
    fn new(args: Vec<String>, env: HashMap<String, String>) -> Self;
    fn run(&mut self) -> Result<i32, E>;

    fn process() {
        let mut p = Self::new(Vec::from_iter(env::args()), HashMap::from_iter(env::vars()));
        process::exit(match p.run() {
            Result::Ok(code) => code,
            Result::Err(error) => {
                println!("{}", error);
                1
            }
        })
    }
}