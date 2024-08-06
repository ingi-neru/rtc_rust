use lazy_static::lazy_static;
use rand::Rng;
use std::io::{stdin, Read, Write};
use std::net::TcpStream;
use std::sync::Mutex;
use std::thread;

lazy_static! {
    static ref NICKNAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

fn random_number() -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100000, 999999);
    num.to_string()
}

fn join_chat(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut did_try = false;
    let mut nickname = String::new();

    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("{}", response);
                if bytes_read == 0 {
                    println!("Connection closed by the server.");
                    break;
                }

                if response.trim() == "NICK" {
                    let mut current = String::new();
                    if !did_try {
                        println!("Choose a nickname: ");
                    } else {
                        println!("Nickname already taken, choose another: ");
                    }
                    did_try = true;

                    nickname.clear();
                    stdin()
                        .read_line(&mut current)
                        .expect("Failed to read line");
                    nickname = current.trim().to_string();

                    if nickname.is_empty() {
                        let random_num = random_number();
                        nickname = "Anonymous".to_string() + &random_num;
                    }
                    println!("nickname = {}", nickname);
                    stream
                        .write(nickname.as_bytes())
                        .expect("Failed to write to stream");
                }

                if response.trim() == "OK" {
                    println!("Connected as: {}", nickname);
                    break;
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
    write_chat(stream);
}

fn write_chat(mut stream: TcpStream) {
    thread::spawn(|| {
        read_chat(stream);
    });

    loop {
        let mut message = String::new();
        stdin()
            .read_line(&mut message)
            .expect("Failed to read line");
    }
}

fn read_chat(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                let response = String::from_utf8_lossy(&buffer[..bytes_read]);
                println!("{}", response);
                if bytes_read == 0 {
                    println!("Connection closed by the server.");
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

fn main() {
    let stream = TcpStream::connect("127.0.0.1:44454").expect("Failed to connect to server");
    join_chat(stream);
}
