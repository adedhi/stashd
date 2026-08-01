use std::fs::File;
use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::persistence::log_command;
use crate::resp;
use crate::store::{Entry, Store};

pub fn handle_connection(stream: TcpStream, store: Store, log: Arc<Mutex<File>>) {
    let mut writer = stream.try_clone().unwrap();
    let mut reader = BufReader::new(stream);

    loop {
        let command = match resp::read_command(&mut reader) {
            Ok(Some(parts)) if !parts.is_empty() => parts,
            Ok(Some(_)) => continue,
            Ok(None) => break,
            Err(_) => break,
        };

        let cmd_name = command[0].to_uppercase();
        let args: Vec<&str> = command.iter().map(|s| s.as_str()).collect();

        let plain_command = command.join(" ");

        let response = match(cmd_name.as_str(), args.as_slice()) {
            ("SET", [_, key, value]) => {
                let mut map = store.lock().unwrap();
                map.insert(key.to_string(), Entry { value: value.to_string(), expires_at: None });
                log_command(&log, &plain_command);
                resp::simple_string("OK")
            }
            ("GET", [_, key]) => {
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
                    Some(value) => resp::bulk_string(&value),
                    None => resp::null(),
                }
            }
            ("DEL", [_, key]) => {
                let mut map = store.lock().unwrap();
                map.remove(*key);
                log_command(&log, &plain_command);
                resp::simple_string("OK")
            }
            ("EXPIRE", [_, key, seconds]) => {
                let mut map = store.lock().unwrap();
                if let Some(entry) = map.get_mut(*key) {
                    let secs: u64 = seconds.parse().unwrap_or(0);
                    let expiry = SystemTime::now() + Duration::from_secs(secs);
                    entry.expires_at = Some(expiry);

                    let unix_secs = expiry.duration_since(UNIX_EPOCH).unwrap().as_secs();
                    log_command(&log, &format!("EXPIRE {} {}", key, unix_secs));
                    resp::simple_string("OK")
                } else {
                    resp::error("ERROR no such key")
                }
            }
            _ => resp::error("ERROR unknown command"),
        };

        writer.write_all(response.as_bytes()).unwrap();
    };
}

pub fn run_server(address: &str, store: Store, log: Arc<Mutex<File>>) {
    let listener = TcpListener::bind(address).unwrap();
    println!("Listening on {}", address);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };
        let store = store.clone();
        let log = log.clone();

        thread::spawn(move || {
            handle_connection(stream, store, log);
        });
    }
}
