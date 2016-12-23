extern crate nom;
extern crate http_handler;

use http_handler::server::{Server, Process};

fn main() {
    Server::process()
}