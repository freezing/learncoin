use crate::core::Sha256;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::fmt::{Display, Formatter};

big_array! {BigArray;}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address(Sha256);

impl Address {
    pub fn new(sha256: Sha256) -> Self {
        Self(sha256)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::core::as_hex(&self.0.bytes()[..]))
    }
}
