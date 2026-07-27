use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

pub type Store = Arc<Mutex<HashMap<String, String>>>;

pub fn new_store() -> Store {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn run_server(address: &str, store: Store) {
    let listener = TcpListener::bind(address).unwrap();
    println!("Listening on {}", address);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let store = store.clone();
        thread::spawn(move || {
            handle_connection(stream, store);
        });
    }
}

pub fn handle_connection(mut stream: TcpStream, store: Store) {
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request = request.trim();

    let parts: Vec<&str> = request.split_whitespace().collect();

    let response = match parts.as_slice() {
        ["SET", key, value] => {
            let mut map = store.lock().unwrap();
            map.insert(key.to_string(), value.to_string());
            "OK\n".to_string()
        }
        ["GET", key] => {
            let map = store.lock().unwrap();
            match map.get(*key) {
                Some(value) => format!("{}\n", value),
                None => "NOT FOUND\n".to_string(),
            }
        }
        ["DEL", key] => {
            let mut map = store.lock().unwrap();
            map.remove(*key);
            "OK\n".to_string()
        }
        _ => "ERROR: unknown command\n".to_string(),
    };

    stream.write_all(response.as_bytes()).unwrap();
}
