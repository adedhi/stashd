use std::thread;
use std::time::Duration;

fn main() {
    let store = stashd::new_store();
    let log = stashd::open_log("stashd.log");
    
    stashd::replay_log("stashd.log", &store);

    {
        let store = store.clone();
        let log = log.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(5));
            stashd::compact_log("stashd.log", &log, &store);
        });
    }

    stashd::run_server("0.0.0.0:7878", store, log);
}
