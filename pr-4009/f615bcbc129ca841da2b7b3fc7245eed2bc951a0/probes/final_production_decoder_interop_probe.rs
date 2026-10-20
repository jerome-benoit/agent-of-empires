
#[cfg(test)]
#[test]
fn pr4009_final_production_decoder_interop_probe() {
    for (name, label) in [
        ("published-control-snapshot.json", "published_control"),
        ("latest-snapshot.json", "live_final_producer"),
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/protocol4-interop")
            .join(name);
        let bytes = std::fs::read(path).unwrap();
        let snapshot = parse_snapshot(&bytes).unwrap();
        validate_snapshot(&snapshot).unwrap();
        println!("final_production_consumer_{label}=PASS");
    }
}