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

pub struct Server {
    port: u16,
    host: String,
}

impl Server {
    //    fn request(&mut read:Read) -> Result<HttpMessage, Error> {
    //
    //    }
}


impl Process<Error> for Server {
    fn new(args: Vec<String>, env: HashMap<String, String>) -> Server {
        assert_eq!(args.len(), 1);

        Server {
            port: env.get("PORT").and_then(|value| value.parse().ok()).unwrap_or(8080),
            host: env.get("HOST").unwrap_or(&"0.0.0.0".to_string()).clone(),
        }
    }
    fn run(&mut self) -> Result<i32, Error> {
        let authority = (self.host.as_str(), self.port);
        let listener: TcpListener = try!(TcpListener::bind(authority));
        self.port = try!(listener.local_addr()).port();
        println!("listening on http://{}:{}/", self.host, self.port);

        for stream in listener.incoming() {
            thread::spawn(|| {
                let mut buffer: [u8; 4096] = [0; 4096];
                let mut stream: TcpStream = stream.unwrap();
                let read = stream.read(&mut buffer[..]).unwrap();
                match http_message(&buffer[..read]) {
                    IResult::Done(_, request) => {
                        let mut handler = LogHandler{handler:TestHandler{}};
                        let response = handler.handle(&request);
                        let text = format!("{}", response);
                        let bytes = text.as_bytes();
                        let wrote = stream.write(bytes).unwrap();
                        assert_eq!(bytes.len(), wrote);
                    },
                    IResult::Incomplete(needed) => {
                        println!("Incomplete need {:?}", needed);
                    },
                    IResult::Error(err) => {
                        println!("Error {}", err);
                    },
                }
            });
        }
        Ok(0)
    }
}

trait HttpHandler{
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage;
}

struct TestHandler {}

impl HttpHandler for TestHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        HttpMessage {
            start_line: StartLine::StatusLine(StatusLine { version: HttpVersion { major: 1, minor: 1, }, code: 200, description: "OK" }),
            headers: Headers(vec!(("Content-Type", "text/plain".to_string()), ("Content-Length", "5".to_string()))),
            body: MessageBody::Slice(&b"Hello"[..]),
        }
    }
}

struct LogHandler<H> where H: HttpHandler {
    handler: H,
}

impl <H> HttpHandler for LogHandler<H> where H: HttpHandler {
    fn handle(&mut self, request: &HttpMessage) -> HttpMessage {
        let response = self.handler.handle(request);
        print!("{}{}\n\n\n", request, response);
        response
    }
}