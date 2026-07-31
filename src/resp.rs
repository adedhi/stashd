use std::io::{self, BufRead};

pub fn simple_string(s: &str) -> String {
    format!("+{}\r\n", s)
}

pub fn error(s: &str) -> String {
    format!("-{}\r\n", s)
}

pub fn bulk_string(s: &str) -> String {
    format!("${}\r\n{}\r\n", s.len(), s)
}

pub fn null() -> String {
    "$-1\r\n".to_string()
}

pub fn read_command<R: BufRead>(reader: &mut R) -> io::Result<Option<Vec<String>>> {
    let mut header = String::new();
    let bytes_read = reader.read_line(&mut header)?;

    if bytes_read == 0 {
        return Ok(None);
    }

    let header = header.trim_end();

    if !header.starts_with("*") {
        return Ok(Some(vec![]));
    }

    let count: usize = header[1..].parse().unwrap_or(0);
    let mut parts = Vec::with_capacity(count);

    for _ in 0..count {
        let mut len_line = String::new();
        reader.read_line(&mut len_line)?;
        let len_line = len_line.trim_end();

        if !len_line.starts_with("$") {
            continue;
        }

        let len: usize = len_line[1..].parse().unwrap_or(0);

        let mut buf = vec![0u8; len + 2];
        reader.read_exact(&mut buf)?;

        let value = String::from_utf8_lossy(&buf[..len]).to_string();
        parts.push(value);
    }

    Ok(Some(parts))
}
