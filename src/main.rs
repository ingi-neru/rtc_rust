use lazy_static::lazy_static;
use signal_hook::flag;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
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
impl PartialEq for SharedStream {
    fn eq(&self, other: &Self) -> bool {
        // Compare the fields of SharedStream
        let nickname_1 = self
            .nickname
            .lock()
            .expect("Error when trying to lock nickname");
        let nickname_2 = other
            .nickname
            .lock()
            .expect("Error when trying to lock nickname");

        nickname_1.clone() == nickname_2.clone()
    }
}

fn remove_stream(stream: Arc<SharedStream>) {
    let mut streams = STREAMS.lock().expect("Error when locking streams");

    let index = streams
        .iter()
        .position(|x| Arc::ptr_eq(x, &stream))
        .unwrap();
    streams.remove(index);
}

fn check_if_nickname_exists(nickname: String) -> bool {
    let streams = STREAMS.lock().expect("Failed to lock mutex");
    for stream in streams.iter() {
        let stream_nickname = stream
            .nickname
            .lock()
            .expect("Error locking nickname mutex");
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
            thread::spawn(move || {
                handle_client_messages(stream_clone);
            });
            break;
        } else {
            message = "NICK";
        }
    }
}

fn broadcast_message(message: String, current_stream_ptr: *const Mutex<TcpStream>) {
    let streams = STREAMS.lock().expect("Failed to lock mutex");

    for other_stream in streams.iter() {
        let other_stream_ptr = Arc::as_ptr(other_stream) as *const Mutex<TcpStream>;
        if other_stream_ptr != current_stream_ptr {
            let mut other_write_lock = other_stream
                .write_stream
                .lock()
                .expect("Failed to lock write stream");

            if let Err(e) = other_write_lock.write_all(message.as_bytes()) {
                eprintln!("Failed to write to stream: {}", e);
            }
        }
    }
}

fn send_message(message: String, stream: Arc<Mutex<TcpStream>>) {
    stream
        .lock()
        .expect("Failed to lock stream")
        .write_all(message.as_bytes())
        .expect("Failed to write to stream");
}

fn close_server(current_stream_ptr: *const Mutex<TcpStream>) {
    let message = String::from("SERVER CLOSED");
    broadcast_message(message, current_stream_ptr);
    println!("Closing server");
}

fn handle_client_messages(shared_stream: Arc<SharedStream>) {
    let mut buffer = [0; 1024];
    let term = Arc::new(AtomicBool::new(false));
    let stream_as_ptr = Arc::as_ptr(&shared_stream) as *const Mutex<TcpStream>;
    flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))
        .expect("Failed to register signal");

    while !term.load(Ordering::Relaxed) {
        let mut stream_lock = shared_stream
            .read_stream
            .lock()
            .expect("Failed to lock stream");

        match stream_lock.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    continue;
                }

                let response = String::from_utf8_lossy(&buffer)
                    .trim()
                    .replace("\0", "")
                    .to_string();
                if response.is_empty() {
                    continue;
                }

                let formatted_response = {
                    let nickname_guard = shared_stream
                        .nickname
                        .lock()
                        .expect("Failed to lock nickname");
                    format!("<{}>: {}", &*nickname_guard, response)
                };
                if response.trim() == "/exit" {
                    let nickname_guard = shared_stream
                        .nickname
                        .lock()
                        .expect("Failed to lock nickname");
                    println!("{} left the chat", &*nickname_guard);
                    send_message(String::from("EXIT"), shared_stream.write_stream.clone());
                    remove_stream(shared_stream.clone());
                } else {
                    broadcast_message(formatted_response, stream_as_ptr);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }
    let empty_ptr = std::ptr::null() as *const Mutex<TcpStream>;
    close_server(empty_ptr);
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
