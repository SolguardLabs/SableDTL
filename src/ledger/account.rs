use serde::{Deserialize, Serialize};

use crate::ids::AccountId;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum AccountClass {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
    ContraRevenue,
    Clearing,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChartAccount {
    pub id: AccountId,
    pub code: String,
    pub name: String,
    pub class: AccountClass,
    pub normal_debit: bool,
    pub enabled: bool,
}

impl ChartAccount {
    pub fn new(
        id: AccountId,
        code: impl Into<String>,
        name: impl Into<String>,
        class: AccountClass,
    ) -> Self {
        let normal_debit = matches!(
            class,
            AccountClass::Asset | AccountClass::Expense | AccountClass::ContraRevenue
        );
        Self {
            id,
            code: code.into(),
            name: name.into(),
            class,
            normal_debit,
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}
