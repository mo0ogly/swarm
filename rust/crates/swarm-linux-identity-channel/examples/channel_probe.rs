//! Test-only stdin socket consumer. Never a daemon/admission or normal CLI.
#![forbid(unsafe_code)]
use std::io::{self, Write};
use swarm_linux_identity_channel::configure;
use zeroize::Zeroizing;
/// Configure inherited stdin before announcing readiness. The driver releases
/// one synthetic packet afterwards. Print only expected case labels, no secrets,
/// credential values or FD numbers; close accepted pidfd explicitly.
fn main() {
    let expect = std::env::args().nth(1).expect("explicit expected outcome");
    assert!(expect == "valid" || expect == "invisible");
    let input = io::stdin();
    let mut channel = configure(&input).expect("private seqpacket input");
    println!("CHANNEL_READY");
    io::stdout().flush().unwrap();
    let before = std::fs::read_dir("/proc/self/fd").unwrap().count();
    match channel.receive(Zeroizing::new(vec![0; 64])) {
        Ok(packet) => {
            assert_eq!(expect, "valid");
            assert_eq!(packet.data(), b"witness");
            assert!(packet.credentials().0 > 0);
            packet.namespaces().unwrap().close().unwrap();
            packet.close().unwrap();
            println!("CHANNEL_VALID");
        }
        Err(error) => {
            assert_eq!(expect, "invisible");
            assert_eq!(error.code(), "identity-channel-unattributed");
            assert!(error.causes().is_empty());
            println!("CHANNEL_INVISIBLE_REFUSED");
        }
    }
    assert_eq!(std::fs::read_dir("/proc/self/fd").unwrap().count(), before);
    println!("CHANNEL_NO_FD_LEAK");
}
