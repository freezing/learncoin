use sha2::Digest;
use std::fmt::{Display, Formatter};

const SHA256_BYTE_COUNT: usize = 32;

/// Sha-256 is a 256-bit array or 32 bytes.
/// It provides an API to display as hex-encoded string and parse it from a hex-encoded string.
#[derive(Copy, Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct Sha256([u8; SHA256_BYTE_COUNT]);

impl Sha256 {
    pub const fn from_raw(raw_bytes: [u8; SHA256_BYTE_COUNT]) -> Self {
        Self(raw_bytes)
    }

    pub fn digest(data: &[u8]) -> Self {
        // We are going to discuss the sha256 API when we implement our own version.
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        assert_eq!(result.len(), SHA256_BYTE_COUNT);
        let mut output = [0; SHA256_BYTE_COUNT];
        for (i, byte) in result.iter().enumerate() {
            output[i] = *byte;
        }
        Sha256::from_raw(output)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.as_slice())
    }

    pub fn from_hex(s: &str) -> Result<Self, String> {
        match hex::decode(&s) {
            Ok(bytes) => {
                let mut sha = [0; SHA256_BYTE_COUNT];
                if bytes.len() == SHA256_BYTE_COUNT {
                    for i in 0..SHA256_BYTE_COUNT {
                        sha[i] = *bytes.get(i).unwrap();
                    }
                    Ok(Sha256::from_raw(sha))
                } else {
                    Err(format!(
                        "Invalid SHA-256 length. Expected: {} but got: {} in: {}",
                        SHA256_BYTE_COUNT,
                        bytes.len(),
                        s
                    ))
                }
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Display for Sha256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}
