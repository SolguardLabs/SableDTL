use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::{SableError, SableResult};
use crate::ids::{AccountId, PartyId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CounterpartyRole {
    Merchant,
    Buyer,
    Processor,
    Treasury,
    ClearingHouse,
    Auditor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BankAccount {
    pub institution: String,
    pub routing_code: String,
    pub account_ref: String,
    pub currency: String,
    pub enabled: bool,
}

impl BankAccount {
    pub fn new(
        institution: impl Into<String>,
        routing_code: impl Into<String>,
        account_ref: impl Into<String>,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            institution: institution.into(),
            routing_code: routing_code.into(),
            account_ref: account_ref.into(),
            currency: currency.into(),
            enabled: true,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Counterparty {
    pub id: PartyId,
    pub legal_name: String,
    pub role: CounterpartyRole,
    pub settlement_account: AccountId,
    pub receivable_account: AccountId,
    pub payable_account: AccountId,
    pub bank_accounts: Vec<BankAccount>,
    pub active: bool,
}

impl Counterparty {
    pub fn new(
        id: PartyId,
        legal_name: impl Into<String>,
        role: CounterpartyRole,
        settlement_account: AccountId,
        receivable_account: AccountId,
        payable_account: AccountId,
    ) -> Self {
        Self {
            id,
            legal_name: legal_name.into(),
            role,
            settlement_account,
            receivable_account,
            payable_account,
            bank_accounts: Vec::new(),
            active: true,
        }
    }

    pub fn with_bank_account(mut self, account: BankAccount) -> Self {
        self.bank_accounts.push(account);
        self
    }

    pub fn primary_bank_account(&self) -> Option<&BankAccount> {
        self.bank_accounts.iter().find(|account| account.enabled)
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CounterpartyBook {
    parties: BTreeMap<PartyId, Counterparty>,
}

impl CounterpartyBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, party: Counterparty) -> SableResult<()> {
        if self.parties.contains_key(&party.id) {
            return Err(SableError::DuplicateId(party.id.to_string()));
        }
        self.parties.insert(party.id.clone(), party);
        Ok(())
    }

    pub fn get(&self, id: &PartyId) -> SableResult<&Counterparty> {
        self.parties
            .get(id)
            .ok_or_else(|| SableError::CounterpartyNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &PartyId) -> SableResult<&mut Counterparty> {
        self.parties
            .get_mut(id)
            .ok_or_else(|| SableError::CounterpartyNotFound(id.clone()))
    }

    pub fn by_role(&self, role: CounterpartyRole) -> Vec<&Counterparty> {
        self.parties
            .values()
            .filter(|party| party.role == role)
            .collect()
    }

    pub fn active_parties(&self) -> Vec<&Counterparty> {
        self.parties.values().filter(|party| party.active).collect()
    }

    pub fn values(&self) -> impl Iterator<Item = &Counterparty> {
        self.parties.values()
    }
}
