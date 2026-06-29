//! Networking primitives: a shared HTTP client, text/byte fetches, and SHA-256
//! verification of downloaded artifacts.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// User agent sent with every request.
const USER_AGENT: &str = concat!("aspid/", env!("CARGO_PKG_VERSION"));

/// The process-wide, lazily-initialised HTTP client (connection pooling, rustls).
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .use_rustls_tls()
            .build()
            .expect("failed to build HTTP client")
    })
}

/// Fetch a URL as UTF-8 text (used for the ModLinks/ApiLinks XML feeds).
pub async fn fetch_text(url: &str) -> Result<String> {
    let resp = client().get(url).send().await?.error_for_status()?;
    Ok(resp.text().await?)
}

/// Download a URL fully into memory. Mod and API archives are small (a few MB), so
/// buffering is fine and keeps verification simple.
pub async fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let resp = client().get(url).send().await?.error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

/// Lower-case hex SHA-256 of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Verify `bytes` against an expected SHA-256 (case-insensitive hex).
pub fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err(Error::ChecksumMismatch {
            expected: expected.trim().to_lowercase(),
            actual,
        })
    }
}

/// Download a URL and verify its checksum in one step.
pub async fn download_verified(url: &str, expected_sha256: &str) -> Result<Vec<u8>> {
    let bytes = download_bytes(url).await?;
    verify_sha256(&bytes, expected_sha256)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        // SHA-256 of "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn verify_is_case_insensitive() {
        let upper = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert!(verify_sha256(b"abc", upper).is_ok());
        assert!(verify_sha256(b"abc", "deadbeef").is_err());
    }
}
