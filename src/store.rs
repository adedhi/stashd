use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub struct Entry {
    pub value: String,
    pub expires_at: Option<SystemTime>,
}

pub type Store = Arc<Mutex<HashMap<String, Entry>>>;

pub fn new_store() -> Store {
    Arc::new(Mutex::new(HashMap::new()))
}
