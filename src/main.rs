fn main() {
    let store = stashd::new_store();
    let log = stashd::open_log("stashd.log");
    
    stashd::replay_log("stashd.log", &store);
    stashd::run_server("0.0.0.0:7878", store, log);
}
