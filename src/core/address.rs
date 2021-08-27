use std::fmt::{Display, Formatter};

pub struct Address(String);

impl From<String> for Address {
    fn from(value: String) -> Self {
        Address(value)
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
