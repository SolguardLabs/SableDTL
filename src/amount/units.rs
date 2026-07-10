use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

use crate::error::{SableError, SableResult};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub struct Money {
    cents: i64,
}

impl Money {
    pub const ZERO: Money = Money { cents: 0 };

    pub const fn from_cents(cents: i64) -> Self {
        Self { cents }
    }

    pub const fn cents(self) -> i64 {
        self.cents
    }

    pub fn dollars(units: i64) -> Self {
        Self {
            cents: units.saturating_mul(100),
        }
    }

    pub fn checked_add(self, other: Money) -> SableResult<Money> {
        self.cents
            .checked_add(other.cents)
            .map(Money::from_cents)
            .ok_or(SableError::ArithmeticOverflow)
    }

    pub fn checked_sub(self, other: Money) -> SableResult<Money> {
        self.cents
            .checked_sub(other.cents)
            .map(Money::from_cents)
            .ok_or(SableError::ArithmeticOverflow)
    }

    pub fn checked_mul_i64(self, factor: i64) -> SableResult<Money> {
        self.cents
            .checked_mul(factor)
            .map(Money::from_cents)
            .ok_or(SableError::ArithmeticOverflow)
    }

    pub fn checked_abs(self) -> SableResult<Money> {
        self.cents
            .checked_abs()
            .map(Money::from_cents)
            .ok_or(SableError::ArithmeticOverflow)
    }

    pub fn min(self, other: Money) -> Money {
        if self <= other { self } else { other }
    }

    pub fn max(self, other: Money) -> Money {
        if self >= other { self } else { other }
    }

    pub fn is_zero(self) -> bool {
        self.cents == 0
    }

    pub fn is_positive(self) -> bool {
        self.cents > 0
    }

    pub fn is_negative(self) -> bool {
        self.cents < 0
    }

    pub fn saturating_sub_floor_zero(self, other: Money) -> Money {
        Money::from_cents(self.cents.saturating_sub(other.cents).max(0))
    }

    pub fn apply_bps(self, bps: Bps) -> SableResult<Money> {
        let numerator = (self.cents as i128)
            .checked_mul(bps.value() as i128)
            .ok_or(SableError::ArithmeticOverflow)?;
        let rounded = div_round_nearest(numerator, 10_000);
        i64::try_from(rounded)
            .map(Money::from_cents)
            .map_err(|_| SableError::ArithmeticOverflow)
    }

    pub fn percent_of(self, numerator: i64, denominator: i64) -> SableResult<Money> {
        if denominator == 0 {
            return Err(SableError::InvalidRatio);
        }
        let raw = (self.cents as i128)
            .checked_mul(numerator as i128)
            .ok_or(SableError::ArithmeticOverflow)?;
        let rounded = div_round_nearest(raw, denominator as i128);
        i64::try_from(rounded)
            .map(Money::from_cents)
            .map_err(|_| SableError::ArithmeticOverflow)
    }

    pub fn format_major(self) -> String {
        let sign = if self.cents < 0 { "-" } else { "" };
        let abs = self.cents.unsigned_abs();
        format!("{sign}{}.{:02}", abs / 100, abs % 100)
    }
}

impl Add for Money {
    type Output = Money;

    fn add(self, rhs: Money) -> Self::Output {
        Money::from_cents(self.cents + rhs.cents)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        self.cents += rhs.cents;
    }
}

impl Sub for Money {
    type Output = Money;

    fn sub(self, rhs: Money) -> Self::Output {
        Money::from_cents(self.cents - rhs.cents)
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        self.cents -= rhs.cents;
    }
}

impl Neg for Money {
    type Output = Money;

    fn neg(self) -> Self::Output {
        Money::from_cents(-self.cents)
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Money::ZERO, |acc, value| acc + value)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Self {
        iter.fold(Money::ZERO, |acc, value| acc + *value)
    }
}

impl Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_major())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Bps(u16);

impl Bps {
    pub const ZERO: Bps = Bps(0);
    pub const ONE_PERCENT: Bps = Bps(100);
    pub const FULL: Bps = Bps(10_000);

    pub fn new(value: u16) -> SableResult<Self> {
        if value <= 10_000 {
            Ok(Self(value))
        } else {
            Err(SableError::InvalidBasisPoints(value))
        }
    }

    pub const fn value(self) -> u16 {
        self.0
    }

    pub fn checked_add(self, other: Bps) -> SableResult<Bps> {
        Bps::new(
            self.0
                .checked_add(other.0)
                .ok_or(SableError::ArithmeticOverflow)?,
        )
    }

    pub fn checked_sub(self, other: Bps) -> SableResult<Bps> {
        if other.0 > self.0 {
            return Err(SableError::InvalidRatio);
        }
        Bps::new(self.0 - other.0)
    }
}

impl Display for Bps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} bps", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Ratio {
    numerator: i64,
    denominator: i64,
}

impl Ratio {
    pub fn new(numerator: i64, denominator: i64) -> SableResult<Self> {
        if denominator == 0 {
            return Err(SableError::InvalidRatio);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn numerator(self) -> i64 {
        self.numerator
    }

    pub const fn denominator(self) -> i64 {
        self.denominator
    }

    pub fn apply(self, amount: Money) -> SableResult<Money> {
        amount.percent_of(self.numerator, self.denominator)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Allocation {
    pub label: String,
    pub amount: Money,
}

impl Allocation {
    pub fn new(label: impl Into<String>, amount: Money) -> Self {
        Self {
            label: label.into(),
            amount,
        }
    }
}

pub fn allocate_by_weights(
    total: Money,
    weights: &[(String, i64)],
) -> SableResult<Vec<Allocation>> {
    let denominator: i64 = weights.iter().map(|(_, weight)| *weight).sum();
    if denominator <= 0 {
        return Err(SableError::InvalidRatio);
    }

    let mut allocated = Money::ZERO;
    let mut rows = Vec::with_capacity(weights.len());
    for (index, (label, weight)) in weights.iter().enumerate() {
        let amount = if index + 1 == weights.len() {
            total - allocated
        } else {
            total.percent_of(*weight, denominator)?
        };
        allocated += amount;
        rows.push(Allocation::new(label.clone(), amount));
    }
    Ok(rows)
}

fn div_round_nearest(numerator: i128, denominator: i128) -> i128 {
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bps_application_rounds_to_nearest_cent() {
        let amount = Money::from_cents(10_005);
        let fee = amount.apply_bps(Bps::new(25).unwrap()).unwrap();
        assert_eq!(fee.cents(), 25);
    }

    #[test]
    fn allocations_preserve_total() {
        let rows = allocate_by_weights(
            Money::from_cents(100),
            &[
                ("a".to_string(), 1),
                ("b".to_string(), 1),
                ("c".to_string(), 1),
            ],
        )
        .unwrap();
        assert_eq!(
            rows.iter().map(|row| row.amount).sum::<Money>(),
            Money::from_cents(100)
        );
    }
}
