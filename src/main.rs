use lazy_static::lazy_static;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

lazy_static! {
    static ref STREAMS: Mutex<Vec<Arc<SharedStream>>> = Mutex::new(Vec::new());
}

struct SharedStream {
    read_stream: Arc<Mutex<TcpStream>>,
    write_stream: Arc<Mutex<TcpStream>>,
    nickname: Mutex<String>,
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
            nickname: Mutex::new(String::new()),
        }
    }
}

fn check_if_nickname_exists(nickname: String) -> bool {
    let streams = STREAMS.lock().expect("Failed to lock mutex");
    println!("streams currently: {}", streams.len());
    for stream in streams.iter() {
        let stream_nickname = stream.nickname.lock().expect("Error locking nickname");
        if *stream_nickname == nickname {
            return true;
        }
    }
    return false;
}

fn join_client(shared_stream: Arc<SharedStream>) {
    let mut buffer = [0; 1024];
    let mut message = "NICK";
    let mut client_read_lock = shared_stream
        .read_stream
        .lock()
        .expect("Failed to lock stream");
    let mut client_write_lock = shared_stream
        .write_stream
        .lock()
        .expect("Failed to lock stream");
    loop {
        client_read_lock
            .write_all(message.as_bytes())
            .expect("Failed to write to stream");
        client_write_lock
            .read(&mut buffer)
            .expect("Failed to read from stream");
        let nickname = String::from_utf8_lossy(&buffer).trim().to_string();
        if !check_if_nickname_exists(nickname.clone()) {
            println!("im stuck!");
            let stream_clone = Arc::clone(&shared_stream);
            {
                let mut stream_nickname_lock = shared_stream
                    .nickname
                    .lock()
                    .expect("Failed to lock nickname");
                *stream_nickname_lock = nickname.clone();
            }
            println!("{} joined the chat", nickname);
            message = "OK";
            client_write_lock
                .write_all(message.as_bytes())
                .expect("Failed to write to stream");
            let mut streams = STREAMS.lock().expect("Failed to lock mutex");
            streams.push(shared_stream.clone());
            println!("stream pushed");
            thread::spawn(move || {
                handle_client_messages(stream_clone);
            });
            break;
        } else {
            message = "NICK";
        }
    }
}

fn handle_client_messages(shared_stream: Arc<SharedStream>) {
    let mut buffer = [0; 1024];
    let current_stream_ptr = Arc::as_ptr(&shared_stream) as *const Mutex<TcpStream>;
    loop {
        let mut stream_lock = shared_stream
            .read_stream
            .lock()
            .expect("Failed to lock stream");
        match stream_lock.read(&mut buffer) {
            Ok(bytes_read) => {
                println!("Bytes read: {}", bytes_read);
                let mut response = String::from_utf8_lossy(&buffer).trim().to_string();
                if bytes_read != 0 {
                    {
                        let nickname_guard = shared_stream
                            .nickname
                            .lock()
                            .expect("Failed to lock nickname");
                        let nickname = &*nickname_guard;
                        response = format!("<{}>: {}", nickname, response);
                    }
                    let streams = STREAMS.lock().expect("Failed to lock mutex");
                    for other_stream in streams.iter() {
                        let other_stream_ptr = Arc::as_ptr(other_stream) as *const Mutex<TcpStream>;
                        if other_stream_ptr != current_stream_ptr {
                            let mut other_write_lock = other_stream
                                .write_stream
                                .lock()
                                .expect("Failed to lock write stream");
                            other_write_lock
                                .write_all(response.as_bytes())
                                .expect("Failed to write to stream");
                        }
                    }
                } else {
                    println!("Connection closed by the client.");
                    break;
                }
                if response == "exit" {
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
    let listener = TcpListener::bind("127.0.0.1:44454").expect("Failed to bind to address");
    println!("Listening on port 44454");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    join_client(Arc::new(SharedStream::new(stream)));
                });
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
