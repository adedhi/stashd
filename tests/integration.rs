use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn send_and_read(stream: &mut TcpStream, command: &str) -> String {
    stream.write_all(command.as_bytes()).unwrap();
    let mut buffer = [0; 512];
    let bytes_read = stream.read(&mut buffer).unwrap();
    String::from_utf8_lossy(&buffer[..bytes_read]).to_string()
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
            let command = format!("SET key{} value{}\n", i, i);
            let response = send_and_read(&mut stream, &command);
            assert_eq!(response.trim(), "OK");
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

    send_and_read(&mut stream, "SET foo bar\n");
    send_and_read(&mut stream, "EXPIRE foo 1\n");

    // Immediately after creation, GET should still return the value
    let response = send_and_read(&mut stream, "GET foo\n");
    assert_eq!(response.trim(), "bar");

    // Wait past the expiration, then check again
    thread::sleep(Duration::from_secs(2));
    let response = send_and_read(&mut stream, "GET foo\n");
    assert_eq!(response.trim(), "NOT FOUND");
}
