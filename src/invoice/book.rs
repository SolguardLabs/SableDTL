use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Money;
use crate::error::{SableError, SableResult};
use crate::ids::{InvoiceId, PartyId};
use crate::invoice::{Invoice, InvoiceStatus};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct InvoiceBook {
    invoices: BTreeMap<InvoiceId, Invoice>,
}

impl InvoiceBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, invoice: Invoice) -> SableResult<()> {
        if self.invoices.contains_key(&invoice.id) {
            return Err(SableError::DuplicateId(invoice.id.to_string()));
        }
        self.invoices.insert(invoice.id.clone(), invoice);
        Ok(())
    }

    pub fn issue(&mut self, id: &InvoiceId) -> SableResult<()> {
        self.get_mut(id)?.issue()
    }

    pub fn get(&self, id: &InvoiceId) -> SableResult<&Invoice> {
        self.invoices
            .get(id)
            .ok_or_else(|| SableError::InvoiceNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &InvoiceId) -> SableResult<&mut Invoice> {
        self.invoices
            .get_mut(id)
            .ok_or_else(|| SableError::InvoiceNotFound(id.clone()))
    }

    pub fn apply_receipt(&mut self, id: &InvoiceId, amount: Money) -> SableResult<()> {
        self.get_mut(id)?.apply_receipt(amount)
    }

    pub fn apply_credit_memo(&mut self, id: &InvoiceId, amount: Money) -> SableResult<()> {
        self.get_mut(id)?.apply_credit_memo(amount)
    }

    pub fn apply_debit_memo(&mut self, id: &InvoiceId, amount: Money) -> SableResult<()> {
        self.get_mut(id)?.apply_debit_memo(amount)
    }

    pub fn close_invoice(&mut self, id: &InvoiceId, day: u32) -> SableResult<()> {
        self.get_mut(id)?.close(day)
    }

    pub fn values(&self) -> impl Iterator<Item = &Invoice> {
        self.invoices.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Invoice> {
        self.invoices.values_mut()
    }

    pub fn by_buyer(&self, buyer: &PartyId) -> Vec<&Invoice> {
        self.invoices
            .values()
            .filter(|invoice| &invoice.buyer == buyer)
            .collect()
    }

    pub fn by_merchant(&self, merchant: &PartyId) -> Vec<&Invoice> {
        self.invoices
            .values()
            .filter(|invoice| &invoice.merchant == merchant)
            .collect()
    }

    pub fn open_accounting_total(&self) -> Money {
        self.invoices
            .values()
            .map(Invoice::accounting_outstanding)
            .sum()
    }

    pub fn open_presentation_total(&self) -> Money {
        self.invoices
            .values()
            .map(Invoice::presentation_outstanding)
            .sum()
    }

    pub fn collected_total(&self) -> Money {
        self.invoices
            .values()
            .map(|invoice| invoice.receipts_applied)
            .sum()
    }

    pub fn memo_total(&self) -> Money {
        self.invoices
            .values()
            .map(|invoice| invoice.credit_memos_applied - invoice.debit_memos_applied)
            .sum()
    }

    pub fn eligible_for_close(&self, day: u32) -> Vec<&Invoice> {
        self.invoices
            .values()
            .filter(|invoice| {
                invoice.issue_day <= day
                    && matches!(
                        invoice.status,
                        InvoiceStatus::Paid | InvoiceStatus::PartiallyCollected
                    )
                    && !invoice.has_accounting_balance()
            })
            .collect()
    }

    pub fn count_by_status(&self, status: InvoiceStatus) -> usize {
        self.invoices
            .values()
            .filter(|invoice| invoice.status == status)
            .count()
    }

    pub fn len(&self) -> usize {
        self.invoices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.invoices.is_empty()
    }
}
