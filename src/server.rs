extern crate nom;
extern crate std;

use std::env;
use std::iter::FromIterator;
use std::collections::HashMap;
use std::process;
use std::io::{Error, Read, Write};
use std::net::{TcpStream, TcpListener};
use std::thread;
use nom::IResult;
use grammar::http_message;
use ast::*;

pub trait Process<E> where Self: std::marker::Sized, E: std::error::Error {
    fn new(args: Vec<String>, env: HashMap<String, String>) -> Self;
    fn run(&self) -> Result<i32, E>;

    fn process() {
        let p = Self::new(Vec::from_iter(env::args()), HashMap::from_iter(env::vars()));
        process::exit(match p.run() {
            Result::Ok(code) => code,
            Result::Err(error) => {
                println!("{}", error);
                1
            }
        })
    }
}

pub struct Server {
    port: u16,
    host: String,
}

impl Server {}


impl Process<Error> for Server {
    fn new(args: Vec<String>, env: HashMap<String, String>) -> Server {
        assert_eq!(args.len(), 1);
        Server {
            port: env.get("PORT").and_then(|value| value.parse().ok()).unwrap_or(8080),
            host: env.get("HOST").unwrap_or(&"127.0.0.1".to_string()).clone()
        }
    }
    fn run(&self) -> Result<i32, Error> {
        let host = format!("{}:{}", self.host, self.port);
        let listener = try!(TcpListener::bind(host.as_str()));
        println!("listening on http://{}/", host);
        for stream in listener.incoming() {
            thread::spawn(|| {
                let mut buffer: [u8; 4096] = [0; 4096];
                let mut stream: TcpStream = stream.unwrap();
                let read = stream.read(&mut buffer[..]).unwrap();
                if let IResult::Done(_, request) = http_message(&buffer[..read]) {
                    let response = HttpMessage {
                        start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
                        headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", "5".to_string()))),
                        body: MessageBody::Slice(&b"Hello"[..]),
                    };
                    let text = format!("{}", response);
                    print!("{}{}\n\n\n", request, text);
                    let bytes = text.as_bytes();
                    let wrote = stream.write(bytes).unwrap();
                    assert_eq!(bytes.len(), wrote);
                }
            });
        }
        Ok(0)
    }
}