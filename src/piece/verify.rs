use sha1::{Digest, Sha1};

/// Verifies that the downloaded piece data matches its expected SHA1 hash.
///
/// # Arguments
/// * `data` - The assembled piece data
/// * `expected_hash` - The expected 20-byte SHA1 hash from the torrent file
///
/// # Returns
/// `true` if the hash matches, `false` otherwise
pub fn verify_piece(data: &[u8], expected_hash: &[u8; 20]) -> bool {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let actual_hash: [u8; 20] = hasher.finalize().into();
    actual_hash == *expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_piece_valid() {
        let data = b"Hello, World!";
        let mut hasher = Sha1::new();
        hasher.update(data);
        let expected: [u8; 20] = hasher.finalize().into();

        assert!(verify_piece(data, &expected));
    }

    #[test]
    fn test_verify_piece_invalid() {
        let data = b"Hello, World!";
        let wrong_hash = [0u8; 20];

        assert!(!verify_piece(data, &wrong_hash));
    }

    #[test]
    fn test_verify_piece_empty() {
        let data = b"";
        let mut hasher = Sha1::new();
        hasher.update(data);
        let expected: [u8; 20] = hasher.finalize().into();

        assert!(verify_piece(data, &expected));
    }
}
