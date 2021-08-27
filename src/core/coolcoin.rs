use std::fmt::{Display, Formatter};
use std::iter::Sum;
use std::ops::{Add, Sub};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Coolcoin(i64);

impl Coolcoin {
    pub fn new(amount: i64) -> Self {
        Coolcoin::from(amount)
    }

    pub fn zero() -> Self {
        Self::new(0)
    }
}

impl Add for Coolcoin {
    type Output = Coolcoin;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sum<Coolcoin> for Coolcoin {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Self::zero();
        for el in iter {
            sum = sum.add(el);
        }
        sum
    }
}

impl Sub for Coolcoin {
    type Output = Coolcoin;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl From<i64> for Coolcoin {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<i32> for Coolcoin {
    fn from(value: i32) -> Self {
        Self(value as i64)
    }
}

impl Display for Coolcoin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} CLC", self.0)
    }
}
