use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::store::{Entry, Store};

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
            ["PERSIST", key] => {
                if let Some(entry) = map.get_mut(*key) {
                    entry.expires_at = None;
                }
            }
            _ => {}
        }
    }
}

pub fn compact_log(path: &str, log: &Mutex<File>, store: &Store) {
    let mut map = store.lock().unwrap();
    let mut file = log.lock().unwrap();

    map.retain(|_, entry| {
        entry.expires_at.map_or(true, |expiry| expiry > SystemTime::now())
    });

    let temp_path = format!("{}.temp", path);
    let mut temp_file = File::create(&temp_path).unwrap();

    for (key, entry) in map.iter() {
        writeln!(temp_file, "SET {} {}", key, entry.value).unwrap();

        if let Some(expiry) = entry.expires_at {
            if let Ok(unix_secs) = expiry.duration_since(UNIX_EPOCH) {
                writeln!(temp_file, "EXPIRE {} {}", key, unix_secs.as_secs()).unwrap();
            }
        }
    }

    temp_file.flush().unwrap();
    drop(temp_file);

    std::fs::rename(&temp_path, path).unwrap();

    *file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
}
