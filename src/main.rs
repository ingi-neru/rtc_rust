use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn join_client(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    loop {
        stream.read(&mut buf).expect("Failed to read from stream");
        let request = String::from_utf8_lossy(&buf);
        println!("Received: {}", request);
        let response = "Hello Client!".as_bytes();
        stream.write(response).expect("Failed to write to stream");
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:44454").expect("Failed to bind to address");
    println!("Listening on port 8080");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                join_client(stream);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
