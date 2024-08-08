use rand::Rng;
use std::io::{stdin, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn random_number() -> String {
    let mut rng = rand::thread_rng();
    let num: u32 = rng.gen_range(100000, 999999);
    num.to_string()
}

struct SharedStream {
    read_stream: Arc<Mutex<TcpStream>>,
    write_stream: Arc<Mutex<TcpStream>>,
}

impl SharedStream {
    fn new(stream: TcpStream) -> Self {
        let read_stream = Arc::new(Mutex::new(
            stream.try_clone().expect("Failed to clone stream"),
        ));
        let write_stream = Arc::new(Mutex::new(stream));
        SharedStream {
            read_stream,
            write_stream,
        }
    }
}

fn join_chat(shared_stream: Arc<SharedStream>) {
    let mut buffer = [0; 1024];
    let mut did_try = false;
    let mut nickname = String::new();

    loop {
        let bytes_read;
        {
            let mut shared_read_lock = shared_stream
                .read_stream
                .lock()
                .expect("Failed to lock read lock");
            bytes_read = shared_read_lock
                .read(&mut buffer)
                .expect("Failed to read from stream");
        }

        if bytes_read == 0 {
            println!("Connection closed by the server.");
            break;
        }

        let response = String::from_utf8_lossy(&buffer[..bytes_read]);

        if response.trim() == "NICK" {
            let mut current = String::new();
            if !did_try {
                println!("Choose a nickname: ");
            } else {
                println!("Nickname already taken, choose another: ");
            }
            did_try = true;

            stdin()
                .read_line(&mut current)
                .expect("Failed to read line");
            nickname = current.trim().to_string();

            if nickname.is_empty() {
                let random_num = random_number();
                nickname = "Anonymous".to_string() + &random_num;
            }

            {
                let mut shared_write_lock = shared_stream
                    .write_stream
                    .lock()
                    .expect("Failed to lock write lock");
                shared_write_lock
                    .write_all(nickname.as_bytes())
                    .expect("Failed to write to stream");
            }
        }

        if response.trim() == "OK" {
            println!("Connected as: {}", nickname);
            break;
        }

        if response.trim().is_empty() {
            let mut shared_write_lock = shared_stream
                .write_stream
                .lock()
                .expect("Failed to lock write stream");
            if let Err(e) = shared_write_lock.write_all(b"/exit\n") {
                eprintln!("Failed to send message: {}", e);
            }
            break;
        }
    }
    write_chat(shared_stream);
}

fn exit_chat(shared_stream: Arc<SharedStream>) {
    {
        let shared_read_lock = shared_stream
            .read_stream
            .lock()
            .expect("Failed to lock read stream");
        if let Err(e) = shared_read_lock.shutdown(std::net::Shutdown::Both) {
            eprintln!("Failed to close read stream: {}", e);
        }
    }
    //Shutdown write stream
    //{
    //    let shared_write_lock = shared_stream
    //        .write_stream
    //        .lock()
    //        .expect("Failed to lock write stream");
    //    if let Err(e) = shared_write_lock.shutdown(std::net::Shutdown::Both) {
    //        eprintln!("Failed to close write stream: {}", e);
    //    }
    //}
}

fn write_chat(shared_stream: Arc<SharedStream>) {
    // Spawn a new thread for reading chat
    let stream_clone = Arc::clone(&shared_stream);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    thread::spawn(move || {
        read_chat(stream_clone);
        stop_flag_clone.store(true, Ordering::Relaxed);
    });

    while !stop_flag.load(Ordering::Relaxed) {
        let mut message = String::new();
        stdin()
            .read_line(&mut message)
            .expect("Failed to read line");
        let mut stream_lock = shared_stream
            .write_stream
            .lock()
            .expect("Failed to lock write stream");
        if let Err(e) = stream_lock.write_all(message.trim().as_bytes()) {
            eprintln!("Failed to send message: {}", e);
            break;
        }
    }
    println!("Closing connection");
    exit_chat(shared_stream.clone());
}

fn read_chat(shared_stream: Arc<SharedStream>) {
    let mut buffer = [0; 1024];
    loop {
        let bytes_read;
        {
            let mut stream_lock = shared_stream
                .read_stream
                .lock()
                .expect("Failed to lock read stream");
            bytes_read = stream_lock
                .read(&mut buffer)
                .expect("Failed to read from stream");
        }
        if bytes_read == 0 {
            println!("Connection closed by the server.");
            break;
        }

        let response = String::from_utf8_lossy(&buffer).trim().replace("\0", "");
        if response.trim() == "EXIT" {
            break;
        } else if response.trim() == "SERVER CLOSED" {
            println!("Server closed the connection");
        } else {
            print!("{}", response);
        }
    }
}

fn main() {
    let stream = TcpStream::connect("127.0.0.1:44454").expect("Failed to connect to server");
    let shared_stream = Arc::new(SharedStream::new(stream));

    join_chat(shared_stream);
}
