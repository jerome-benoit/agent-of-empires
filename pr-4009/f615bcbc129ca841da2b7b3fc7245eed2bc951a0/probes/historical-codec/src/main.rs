mod session;
mod historical_dto;
mod server {
    pub mod runtime_ws {
        pub const PROTOCOL_VERSION: u16 = 4;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "control snapshot, live snapshot, expected old result");
    let control = std::fs::read(&args[1]).expect("published control snapshot");
    let actual = std::fs::read(&args[2]).expect("actual production snapshot");
    let old_control = historical_dto::parse_snapshot(&control).expect("old decoder accepts published control");
    historical_dto::validate_snapshot(&old_control).expect("old control validates");
    println!("old_consumer_published_control=PASS");
    match historical_dto::parse_snapshot(&actual) {
        Ok(snapshot) => {
            historical_dto::validate_snapshot(&snapshot).expect("live snapshot validates for old consumer");
            println!("old_consumer_live_producer=PASS");
            assert_eq!(args[3], "accept", "old consumer unexpectedly accepted the private extension");
        }
        Err(error) => {
            println!("old_consumer_live_producer=REJECTED: {error:?}");
            assert_eq!(args[3], "reject", "old consumer still rejects the corrected producer");
        }
    }
}
