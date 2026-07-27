fn main() {
    let store = stashd::new_store();
    stashd::run_server("127.0.0.1:7878", store);
}