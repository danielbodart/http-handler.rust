extern crate http_handler;

use std::io::*;
use std::net::*;
use std::thread;
use http_handler::grammar::http_message;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:9123").unwrap();
    println!("listening started, ready to accept");
    for stream in listener.incoming() {
        thread::spawn(|| {
            let mut buffer: [u8; 4096] = [0; 4096];
            let mut stream:TcpStream = stream.unwrap();
            let size = stream.read(&mut buffer[..]).unwrap();
            println!("{:?}", size);
            let result = http_message(&buffer[..size]).unwrap();
            println!("{:?}", result);
        });
    }
}