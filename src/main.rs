extern crate nom;
extern crate http_handler;

use http_handler::server::Server;
use http_handler::api::FileHandler;

fn main() {
    Server::new("0.0.0.0".to_owned(), 8080).handler(||Ok(FileHandler::new(std::env::current_dir()?))).unwrap();
}

//fn port(env: &HashMap<String, String>) -> u16 {
//    env.get("PORT").and_then(|value| value.parse().ok()).unwrap_or(8080)
//}
//
//fn host(env: &HashMap<String, String>) -> String {
//    env.get("HOST").unwrap_or(&"0.0.0.0".to_string()).clone()
//}