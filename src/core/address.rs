use crate::core::Sha256;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::fmt::{Display, Formatter};

big_array! {BigArray;}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address(String);

impl Address {
    pub fn new(address: String) -> Self {
        Self(address)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
