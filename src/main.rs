use lazy_static::lazy_static;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread;

lazy_static! {
    static ref NICKNAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static ref STREAMS: Mutex<Vec<TcpStream>> = Mutex::new(Vec::new());
}

fn join_client(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    let mut message = "NICK";
    loop {
        stream
            .write_all(message.as_bytes())
            .expect("Failed to write to stream");

        stream.read(&mut buf).expect("Failed to read from stream");
        let mut response = String::from_utf8_lossy(&buf).trim().to_string();

        let mut nicknames = NICKNAMES.lock().expect("Failed to lock mutex");

        if !nicknames.contains(&response) {
            println!("{} joined the chat", response);
            message = "OK";
            stream
                .write_all(message.as_bytes())
                .expect("Failed to write to stream");
            nicknames.push(response.clone());
            let mut streams = STREAMS.lock().expect("Failed to lock mutex");
            streams.push(stream.try_clone().expect("Failed to clone stream"));
            handle_client_messages(stream);
            break;
        } else {
            message = "NICK";
            response.clear();
        }
    }
}

fn handle_client_messages(mut stream: TcpStream) {
    let mut buf = [0; 1024];
    println!("hey");
    loop {
        stream.read(&mut buf).expect("Failed to read from stream");
        let response = String::from_utf8_lossy(&buf).trim().to_string();
        println!("{}", response);
        if response == "exit" {
            break;
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:44454").expect("Failed to bind to address");
    println!("Listening on port 44454");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    join_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
