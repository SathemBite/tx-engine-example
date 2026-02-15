use std::{
    fmt::{self, Display},
    ops::{Add, AddAssign, Neg, Sub, SubAssign},
};

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

impl Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TxID(pub u32);

impl Display for TxID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Amount(pub Decimal);

impl Amount {
    pub const ZERO: Self = Self(Decimal::ZERO);

    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn inner(self) -> Decimal {
        self.0
    }
}

impl Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Amount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Amount {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Neg for Amount {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let as_str = match self {
            TransactionType::Deposit => "deposit",
            TransactionType::Withdrawal => "withdrawal",
            TransactionType::Dispute => "dispute",
            TransactionType::Resolve => "resolve",
            TransactionType::Chargeback => "chargeback",
        };
        write!(f, "{as_str}")
    }
}
