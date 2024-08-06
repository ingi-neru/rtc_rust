use std::io::{stdin, stdout, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const OPTIONS: [&str; 6] = [
    "/exit",
    "/clients",
    "/set_broadcast",
    "/recipient",
    "/exit",
    "/help",
];

fn join_chat(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut nickname = String::from("");
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                let data = &buffer[..bytes_read];
                if bytes_read == 0 {
                    println!("Connection closed by the server.");
                    break;
                }

                let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                if data == b"NICK" {
                    println!("Choose a nickname: ");
                    stdout().flush();
                    stdin().read_line(&mut nickname);
                    stream
                        .write(nickname.as_bytes())
                        .expect("Failed to write to stream");
                }

                if response.trim().is_empty() {
                    if let Err(e) = stream.write_all(b"/exit\n") {
                        eprintln!("Failed to send message: {}", e);
                    }
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
}

fn connect_client() {
    let stream = TcpStream::connect("127.0.0.1:44454").expect("Failed to connect to server");
    join_chat(stream);
}

fn read_message(mode: &str, nickname: &str) {}

fn main() {
    thread::spawn(|| connect_client())
        .join()
        .expect("Failed to join thread");
}
