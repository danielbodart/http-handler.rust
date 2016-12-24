extern crate nom;
extern crate http_handler;

use http_handler::server::Server;
use http_handler::process::Process;

fn main() {
    Server::process()
}