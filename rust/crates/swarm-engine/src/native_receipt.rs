//! Atomic receipt framing: extent/hash integrity, never authenticated authority.
#![cfg(unix)]
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    os::unix::fs::MetadataExt,
};
const MAGIC: &[u8] = b"SR1\n";
// u64 has at most20 decimal digits; SHA256 is64 lowercase hexadecimal bytes.
// These are file-format widths, not editable execution/retention policies.
const LENGTH_WIDTH: usize = 20;
const HASH_WIDTH: usize = 64;
const HEADER: usize = MAGIC.len() + LENGTH_WIDTH + 1 + HASH_WIDTH + 1;

/// Static safe diagnostics retain original IO/allocation/parse causes where
/// applicable. Neither file bytes nor private paths are rendered by this layer.
fn invalid(code: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, code)
}

/// Frame complete serialized receipt bytes for the caller's atomic create-only
/// publication. The header records the ACTUAL body extent and SHA, without an
/// inherited global cap or guessed JSON overhead. Checked allocation/conversion
/// failures return before publication; this computes no identity/permission proof.
pub(crate) fn encode(body: &[u8]) -> io::Result<Vec<u8>> {
    let length =
        u64::try_from(body.len()).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let header = format!("SR1\n{length:020}\n{:x}\n", Sha256::digest(body));
    if header.len() != HEADER {
        return Err(invalid("native-receipt-header-representation"));
    }
    let size = HEADER
        .checked_add(body.len())
        .ok_or_else(|| invalid("native-receipt-size-representation"))?;
    let mut raw = Vec::new();
    raw.try_reserve_exact(size).map_err(io::Error::other)?;
    raw.extend_from_slice(header.as_bytes());
    raw.extend_from_slice(body);
    Ok(raw)
}

/// Read one anchored regular receipt handle from offset0. Validate the fixed
/// header and matching current file extent BEFORE allocating the body, then
/// require exact bytes, EOF, hash and stable inode/size/mtime. Unexpected growth,
/// truncation, invalid header/overflow/hash or IO returns refusal, never empty
/// success. Buffer extent comes from the observed serialized receipt rather than
/// a product policy ceiling. Caller owns descriptor closure and domain decoding.
/// This protects against accidental independent corruption and observed races,
/// not a malicious same-UID writer forging both header and body, root substitution
/// before opening, backup rollback, or a deliberately unlimited original payload.
pub(crate) fn read(file: &mut File) -> io::Result<Vec<u8>> {
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(invalid("native-receipt-not-regular"));
    }
    let mut header = [0u8; HEADER];
    file.read_exact(&mut header)?;
    if !header.starts_with(MAGIC)
        || header[MAGIC.len() + LENGTH_WIDTH] != b'\n'
        || header[HEADER - 1] != b'\n'
    {
        return Err(invalid("native-receipt-header-invalid"));
    }
    let digits = &header[MAGIC.len()..MAGIC.len() + LENGTH_WIDTH];
    if !digits.iter().all(u8::is_ascii_digit) {
        return Err(invalid("native-receipt-length-invalid"));
    }
    let length = std::str::from_utf8(digits)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        .parse::<u64>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    if length.checked_add(HEADER as u64) != Some(before.len()) {
        return Err(invalid("native-receipt-extent-mismatch"));
    }
    let hash = &header[MAGIC.len() + LENGTH_WIDTH + 1..HEADER - 1];
    if !hash
        .iter()
        .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
    {
        return Err(invalid("native-receipt-hash-invalid"));
    }
    let size =
        usize::try_from(length).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut body = Vec::new();
    body.try_reserve_exact(size).map_err(io::Error::other)?;
    body.resize(size, 0);
    file.read_exact(&mut body)?;
    let mut extra = [0u8; 1];
    if file.read(&mut extra)? != 0 {
        return Err(invalid("native-receipt-trailing-bytes"));
    }
    let after = file.metadata()?;
    let identity = |m: &std::fs::Metadata| (m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec());
    if identity(&before) != identity(&after) {
        return Err(invalid("native-receipt-changed-during-read"));
    }
    if format!("{:x}", Sha256::digest(&body)).as_bytes() != hash {
        return Err(invalid("native-receipt-body-hash-mismatch"));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    /// Owned isolated regular file, never a mission/private runtime database.
    fn fixture(raw: &[u8]) -> (std::path::PathBuf, File) {
        let path = std::env::temp_dir().join(crate::generate_identifier("receipt126-").unwrap());
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        f.write_all(raw).unwrap();
        f.sync_all().unwrap();
        drop(f);
        let input = File::open(&path).unwrap();
        (path, input)
    }
    /// Positive binary bytes and corruption guards use actual regular handles.
    /// Each injection follows a fresh valid control and retains the original data.
    #[test]
    fn native_receipt_frame_extent_and_hash_refuse_corruption() {
        let original = encode(b"{\"private\":\"owned\"}\0").unwrap();
        for fault in ["hash", "body", "growth", "truncate", "length", "magic"] {
            let (path, mut input) = fixture(&original);
            assert_eq!(read(&mut input).unwrap(), b"{\"private\":\"owned\"}\0");
            drop(input);
            std::fs::remove_file(path).unwrap();
            let mut raw = original.clone();
            match fault {
                "hash" => raw[MAGIC.len() + LENGTH_WIDTH + 1] = b'z',
                "body" => raw[HEADER] ^= 1,
                "growth" => raw.push(b'x'),
                "truncate" => {
                    raw.pop();
                }
                "length" => raw[MAGIC.len()] = b'9',
                "magic" => raw[0] = b'x',
                _ => unreachable!(),
            }
            let (path, mut input) = fixture(&raw);
            let error = read(&mut input).unwrap_err();
            assert!(!error.to_string().contains("private"));
            drop(input);
            std::fs::remove_file(path).unwrap();
        }
    }
    /// Empty body is a framing observation only; the caller's required typed
    /// receipt schema must still refuse it rather than treating it as completion.
    #[test]
    fn native_receipt_empty_extent_is_not_domain_completion() {
        let (path, mut input) = fixture(&encode(b"").unwrap());
        assert!(read(&mut input).unwrap().is_empty());
        drop(input);
        std::fs::remove_file(path).unwrap();
    }
}
