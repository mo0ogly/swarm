//! Experimental harness; calls native product codecs, never Go or task execution.
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("file encode|roundtrip iterations".into());
    }
    let raw = std::fs::read(&args[1])?;
    let mode = args[2].as_str();
    if !matches!(mode, "encode" | "roundtrip") {
        return Err("unknown mode".into());
    }
    let n: u64 = args[3].parse()?;
    if n == 0 {
        return Err("positive iterations".into());
    }
    let value = swarm_engine::decode_request(&raw)?;
    let op = || -> Result<Vec<u8>, swarm_engine::Error> {
        if mode == "roundtrip" {
            swarm_engine::encode_request(&swarm_engine::decode_request(black_box(&raw))?)
        } else {
            swarm_engine::encode_request(black_box(&value))
        }
    };
    let out = op()?;
    let digest: String = Sha256::digest(&out)
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect();
    for _ in 0..n.min(1000) {
        black_box(op()?);
    }
    let start = Instant::now();
    let mut checksum = 0u64;
    for _ in 0..n {
        let b = black_box(op()?);
        checksum = checksum.wrapping_add(b.len() as u64 + b[0] as u64 + b[b.len() - 1] as u64);
        black_box(b);
    }
    let elapsed = start.elapsed().as_nanos();
    use std::io::Write;
    writeln!(
        std::io::stdout().lock(),
        "{}",
        serde_json::json!({"iterations":n,"elapsed_ns":elapsed,"checksum":checksum,"output_bytes":out.len(),"output_sha256":digest})
    )?;
    Ok(())
}
