use std::collections::HashMap;

fn main() {
    let mut store: HashMap<String, String> = HashMap::new();
    
    set(&mut store, "username", "Ad");
    set(&mut store, "language", "Rust");

    match get(&store, "username") {
        Some(value) => println!("username = {}", value),
        None => println!("username not found"),
    }

    match get(&store, "missing_key") {
        Some(value) => println!("missing_key = {}", value),
        None => println!("missing_key not found"),
    }

    delete(&mut store, "language");

    match get(&store, "language") {
        Some(value) => println!("language = {}", value),
        None => println!("language not found (correctly deleted)"),
    }
}

fn set(store: &mut HashMap<String, String>, key: &str, value: &str) {
    store.insert(key.to_string(), value.to_string());
}

fn get<'a>(store: &'a HashMap<String, String>, key: &str) -> Option<&'a String> {
    store.get(key)
}

fn delete(store: &mut HashMap<String, String>, key: &str) {
    store.remove(key);
}
