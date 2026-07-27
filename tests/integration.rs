use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

#[test]
fn handles_concurrent_clients() {
    let store = stashd::new_store();

    thread::spawn(move || {
        stashd::run_server("127.0.0.1:9090", store);
    });

    thread::sleep(Duration::from_millis(100));

    let mut handles = vec![];

    for i in 0..5 {
        handles.push(thread::spawn(move || {
            let mut stream = TcpStream::connect("127.0.0.1:9090").unwrap();
            let command = format!("SET key{} value{}\n", i, i);
            stream.write_all(command.as_bytes()).unwrap();

            let mut buffer = [0; 512];
            let bytes_read = stream.read(&mut buffer).unwrap();
            let response = String::from_utf8_lossy(&buffer[..bytes_read]);
            assert_eq!(response.trim(), "OK");
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}