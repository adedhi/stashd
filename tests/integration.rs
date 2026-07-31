use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn encode_command(parts: &[&str]) -> String {
    let mut encoded = format!("*{}\r\n", parts.len());
    for p in parts {
        encoded += &format!("${}\r\n{}\r\n", p.len(), p);
    }
    encoded
}

fn parse_reply(raw: &str) -> String {
    if raw.starts_with("$-1") {
        "NULL".to_string()
    } else if raw.starts_with("+") || raw.starts_with("-") {
        raw[1..].trim_end().to_string()
    } else if raw.starts_with("$") {
        raw.splitn(2, "\r\n").nth(1).unwrap_or("").trim_end().to_string()
    } else {
        raw.trim().to_string()
    }
}

fn send_and_read(stream: &mut TcpStream, parts: &[&str]) -> String {
    let request = encode_command(parts);
    stream.write_all(request.as_bytes()).unwrap();
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer).unwrap();
    let raw = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
    parse_reply(&raw)
}

#[test]
fn handles_concurrent_clients() {
    let store = stashd::new_store();
    let log = stashd::open_log("stashd.concurrency_test_log");
    let mut handles = vec![];

    thread::spawn(move || {
        stashd::run_server("127.0.0.1:9090", store, log);
    });

    thread::sleep(Duration::from_millis(100));

    for i in 0..5 {
        handles.push(thread::spawn(move || {
            let mut stream = TcpStream::connect("127.0.0.1:9090").unwrap();
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            let response = send_and_read(&mut stream, &["SET", &key, &value]);
            assert_eq!(response, "OK");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn expired_key_is_not_found() {
    let store = stashd::new_store();
    let log = stashd::open_log("stashd.expiry_test_log");
    
    thread::spawn(move || {
        stashd::run_server("127.0.0.1:9091", store, log);
    });

    thread::sleep(Duration::from_millis(100));

    let mut stream = TcpStream::connect("127.0.0.1:9091").unwrap();

    send_and_read(&mut stream, &["SET", "foo", "bar"]);
    send_and_read(&mut stream, &["EXPIRE", "foo", "1"]);

    // Immediately after creation, GET should still return the value
    let response = send_and_read(&mut stream, &["GET", "foo"]);
    assert_eq!(response, "bar");

    // Wait past the expiration, then check again
    thread::sleep(Duration::from_secs(2));
    let response = send_and_read(&mut stream, &["GET", "foo"]);
    assert_eq!(response, "NULL");
}
