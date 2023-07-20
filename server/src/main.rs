use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn handle_conn(mut stream: TcpStream) -> std::io::Result<()> {
    let test = "Hello, World!\n";

    println!(
        "Accepted connection from {}",
        stream.local_addr().unwrap().to_string()
    );

    stream.write(test.as_bytes())?;

    println!(
        "Shutting down connection to {}",
        stream.local_addr().unwrap().to_string()
    );
    Ok(())
}

// ripped from the std::net docs (https://doc.rust-lang.org/std/net/struct.TcpListener.html)
// Presumably returning a result so that we can use the err return shorthand ("?")
fn main() -> std::io::Result<()> {
    let addr = String::from("127.0.0.1:");
    let port = "9001";
    let listener = TcpListener::bind(addr.clone() + port)?;

    println!("Server started, listening on {}{}", addr, port);

    for stream in listener.incoming() {
        handle_conn(stream?)?;
    }
    Ok(())
}
