/// Compute a hex-encoded BLAKE3 hash of the given bytes.
pub fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Compute a prefixed content digest over ordered file entries.
///
/// Each entry is `(relative_path_string, file_contents)`. The entries are
/// hashed in order with a NUL separator after the path and an 0xFF separator
/// after the contents, matching the prior SHA-256 scheme but using BLAKE3.
pub fn content_digest(entries: &[(&str, &[u8])]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (path, contents) in entries {
        hasher.update(path.as_bytes());
        hasher.update(&[0]);
        hasher.update(contents);
        hasher.update(&[0xff]);
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}
