extern crate nom;
extern crate http_handler;

use std::io::*;
use std::net::*;
use std::thread;
use nom::IResult;
use http_handler::grammar::http_message;
use http_handler::ast::*;

fn main() {
    let host = "127.0.0.1:9123";
    let listener = TcpListener::bind(host).unwrap();
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
}