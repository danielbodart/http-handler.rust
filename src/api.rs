use std::io::{Write,Result};
use ast::HttpMessage;

pub trait HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage;
}

pub trait ToWrite {
    fn to_write(&self, write:&mut Write) -> Result<usize>;
}

