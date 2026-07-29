use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Entry {
    pub value: String,
    pub expires_at: Option<SystemTime>,
}

pub type Store = Arc<Mutex<HashMap<String, Entry>>>;

pub fn new_store() -> Store {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn open_log(path: &str) -> Arc<Mutex<File>> {
    Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap()
    ))
}

pub fn log_command(log: &Mutex<File>, command: &str) {
    let mut file = log.lock().unwrap();
    writeln!(file, "{}", command).unwrap();
}

pub fn replay_log(path: &str, store: &Store) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.unwrap();
        let parts: Vec<&str> = line.split_whitespace().collect();

        let mut map = store.lock().unwrap();
        match parts.as_slice() {
            ["SET", key, value] => {
                map.insert(key.to_string(), Entry { value: value.to_string(), expires_at: None });
            }
            ["DEL", key] => {
                map.remove(*key);
            }
            ["EXPIRE", key, seconds] => {
                if let Some(entry) = map.get_mut(*key) {
                    if let Ok(secs) = seconds.parse::<u64>() {
                        entry.expires_at = Some(UNIX_EPOCH + Duration::from_secs(secs));
                    }
                }
            }
            _ => {}
        }
    }
}

pub fn handle_connection(mut stream: TcpStream, store: Store, log: Arc<Mutex<File>>) {
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer).unwrap();

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request = request.trim();

    let parts: Vec<&str> = request.split_whitespace().collect();

    let response = match parts.as_slice() {
        ["SET", key, value] => {
            let mut map = store.lock().unwrap();
            map.insert(key.to_string(), Entry { value: value.to_string(), expires_at: None });
            log_command(&log, request);
            "OK\n".to_string()
        }
        ["GET", key] => {
            let mut map = store.lock().unwrap();
            let mut expired = false;

            let result = match map.get(*key) {
                Some(entry) => {
                    if let Some(expiry) = entry.expires_at {
                        if SystemTime::now() >= expiry {
                            expired = true;
                            None
                        } else {
                            Some(entry.value.clone())
                        }
                    } else {
                        Some(entry.value.clone())
                    }
                }
                None => None,
            };

            if expired {
                map.remove(*key);
            }

            match result {
                Some(value) => format!("{}\n", value),
                None => "NOT FOUND\n".to_string(),
            }
        }
        ["DEL", key] => {
            let mut map = store.lock().unwrap();
            map.remove(*key);
            log_command(&log, request);
            "OK\n".to_string()
        }
        ["EXPIRE", key, seconds] => {
            let mut map = store.lock().unwrap();
            if let Some(entry) = map.get_mut(*key) {
                let secs: u64 = seconds.parse().unwrap_or(0);
                let expiry = SystemTime::now() + Duration::from_secs(secs);
                entry.expires_at = Some(expiry);

                let unix_secs = expiry.duration_since(UNIX_EPOCH).unwrap().as_secs();
                log_command(&log, &format!("EXPIREAT {} {}", key, unix_secs));
                "OK\n".to_string()
            } else {
                "NOT FOUND\n".to_string()
            }
        }
        _ => "ERROR: unknown command\n".to_string(),
    };

    stream.write_all(response.as_bytes()).unwrap();
}

pub fn run_server(address: &str, store: Store, log: Arc<Mutex<File>>) {
    let listener = TcpListener::bind(address).unwrap();
    println!("Listening on {}", address);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let store = store.clone();
        let log = log.clone();

        thread::spawn(move || {
            handle_connection(stream, store, log);
        });
    }
}
