use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct PublicKey(String);

impl PublicKey {
    pub fn new(public_key: String) -> Self {
        Self(public_key)
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
