extern crate nom;
extern crate http_handler;

use http_handler::server::Client;
use http_handler::api::{Request, HttpHandler, LogHandler};
//use http_handler::api::FileHandler;

#[allow(unused_variables)]
#[allow(unused_must_use)]
fn main() {
    let mut request = Request::get("/ip").header("Host", "httpbin.org:80".to_string());
    let mut c = LogHandler::new(Client {});
    c.handle(&mut request, |response| {
        Ok(0)
    });
    //    Server::new("0.0.0.0".to_owned(), 8080).handler(||Ok(FileHandler::new(std::env::current_dir()?))).unwrap();
}

//fn port(env: &HashMap<String, String>) -> u16 {
//    env.get("PORT").and_then(|value| value.parse().ok()).unwrap_or(8080)
//}
//
//fn host(env: &HashMap<String, String>) -> String {
//    env.get("HOST").unwrap_or(&"0.0.0.0".to_string()).clone()
//}