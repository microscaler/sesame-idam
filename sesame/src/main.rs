extern crate may_minihttp;
mod api_keys;
mod mfa;
mod oauth2;
mod org;
mod saml;
pub mod test_utils;
mod user;
use std::io;
extern crate anyhow;
extern crate once_cell;
extern crate reqwest;
extern crate testcontainers;

use may_minihttp::{HttpServer, HttpService, Request, Response};
#[derive(Clone)]
struct HelloWorld;

impl HttpService for HelloWorld {
    fn call(&mut self, _req: Request, res: &mut Response) -> io::Result<()> {
        res.body("Hello, world!");
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let host_port = std::env::var("HOST_PORT").unwrap_or_else(|_| "0.0.0.0:3001".to_string());
    let server = HttpServer(HelloWorld)
        .start(&host_port)
        .map_err(|e| anyhow::anyhow!("Failed to start server: {}", e))?;
    println!("Server started successfully on {}", host_port);
    server
        .join()
        .map_err(|e| anyhow::anyhow!("Server encountered an error: {:?}", e))?;
    Ok(())
}
