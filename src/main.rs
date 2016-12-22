extern crate nom;
extern crate http_handler;

use std::env::{args, vars};
use std::iter::FromIterator;
use std::collections::HashMap;
use std::process::exit;
use std::io::{Error, Read, Write};
use std::net::{TcpStream, TcpListener};
use std::thread;
use nom::IResult;
use http_handler::grammar::http_message;
use http_handler::ast::*;


fn main() {
    Server::process()
}

#[allow(unused_variables)]
#[allow(dead_code)]
trait Process<E>
    where Self: std::marker::Sized, E: std::error::Error {
    fn new(args:Vec<String>, env:HashMap<String, String>) -> Self;
    fn run(&self) -> Result<i32, E>;

    fn process() {
        let p = Self::new(Vec::from_iter(args()), HashMap::from_iter(vars()));
        exit(match p.run() {
            Result::Ok(code) => code,
            Result::Err(error) => {
                println!("{}", error);
                1
            }
        })
    }
}

struct Server {
    port: u16,
    host: String,
}

impl Server {

}

#[allow(unused_variables)]
#[allow(dead_code)]
#[allow(unused_must_use)]
impl Process<Error> for Server {
    fn new(args:Vec<String>, env:HashMap<String, String>) -> Server {
        Server { port: 8080 , host: "127.0.0.1".to_string() }
    }
    fn run(&self) -> Result<i32, Error> {
        let host = format!("{}:{}", self.host, self.port); // TODO fix port
        let listener = try!(TcpListener::bind(host.as_str()));
        println!("listening on http://{}/", host);
        for stream in listener.incoming() {
            thread::spawn(|| {
                let mut buffer: [u8; 4096] = [0; 4096];
                let mut stream: TcpStream = stream.unwrap();
                let size = stream.read(&mut buffer[..]).unwrap();
                if let IResult::Done(_, request) = http_message(&buffer[..size]) {
                    let response = HttpMessage {
                        start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
                        headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", "5".to_string()))),
                        body: MessageBody::Slice(&b"Hello"[..]),
                    };
                    let text = format!("{}", response);
                    print!("{}{}\n\n\n", request, text);
                    stream.write(text.as_bytes());
                }
            });
        }
        Ok(0)
    }
}