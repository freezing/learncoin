use crate::core::Sha256;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct Address(Sha256);

impl Address {
    pub fn new(sha256: Sha256) -> Self {
        Self(sha256)
    }
}

impl Display for Address {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        // write!(f, "{:#?}", self.0)
        todo!("Write as hexadecimal")
    }
}
