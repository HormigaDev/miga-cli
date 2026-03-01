use std::net::TcpStream;
use std::time::Duration;

const CONNECTIVITY_ADDR: &str = "1.1.1.1:53";
const CONNECTIVITY_TIMEOUT_MS: u64 = 1500;

/// Returns `true` if the host can reach the internet (DNS over TCP probe).
pub fn is_online() -> bool {
    TcpStream::connect_timeout(
        &CONNECTIVITY_ADDR.parse().unwrap(),
        Duration::from_millis(CONNECTIVITY_TIMEOUT_MS),
    )
    .is_ok()
}
