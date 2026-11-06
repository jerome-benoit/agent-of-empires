#[test]
fn readiness() {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    out.write_all(b"warm-ready\n").unwrap();
    out.flush().unwrap();
}
