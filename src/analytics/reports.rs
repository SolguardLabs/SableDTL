use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Money;
use crate::ids::{InvoiceId, PartyId};
use crate::invoice::{Invoice, InvoiceBook, InvoiceStatus};
use crate::receipts::ReceiptBook;
use crate::settlement::SettlementBook;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgingBand {
    pub label: String,
    pub min_days: u32,
    pub max_days: Option<u32>,
    pub invoice_count: usize,
    pub accounting_total: Money,
    pub presentation_total: Money,
}

impl AgingBand {
    pub fn new(label: impl Into<String>, min_days: u32, max_days: Option<u32>) -> Self {
        Self {
            label: label.into(),
            min_days,
            max_days,
            invoice_count: 0,
            accounting_total: Money::ZERO,
            presentation_total: Money::ZERO,
        }
    }

    pub fn accepts(&self, days: u32) -> bool {
        days >= self.min_days && self.max_days.is_none_or(|max| days <= max)
    }

    pub fn add(&mut self, invoice: &Invoice) {
        self.invoice_count += 1;
        self.accounting_total += invoice.accounting_outstanding();
        self.presentation_total += invoice.presentation_outstanding();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgingReport {
    pub as_of_day: u32,
    pub bands: Vec<AgingBand>,
    pub total_accounting: Money,
    pub total_presentation: Money,
}

impl AgingReport {
    pub fn build(invoices: &InvoiceBook, as_of_day: u32) -> Self {
        let mut bands = vec![
            AgingBand::new("current", 0, Some(0)),
            AgingBand::new("1-30", 1, Some(30)),
            AgingBand::new("31-60", 31, Some(60)),
            AgingBand::new("61-90", 61, Some(90)),
            AgingBand::new("90+", 91, None),
        ];

        for invoice in invoices.values() {
            if matches!(invoice.status, InvoiceStatus::Voided | InvoiceStatus::Draft) {
                continue;
            }
            let days = invoice.days_past_due(as_of_day);
            if let Some(band) = bands.iter_mut().find(|band| band.accepts(days)) {
                band.add(invoice);
            }
        }

        let total_accounting = bands.iter().map(|band| band.accounting_total).sum();
        let total_presentation = bands.iter().map(|band| band.presentation_total).sum();
        Self {
            as_of_day,
            bands,
            total_accounting,
            total_presentation,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CounterpartyStatement {
    pub party: PartyId,
    pub invoice_count: usize,
    pub gross_total: Money,
    pub receipt_total: Money,
    pub memo_total: Money,
    pub accounting_open: Money,
    pub presentation_open: Money,
    pub invoices: Vec<InvoiceId>,
}

impl CounterpartyStatement {
    pub fn for_buyer(party: PartyId, invoices: &InvoiceBook) -> Self {
        let selected = invoices.by_buyer(&party);
        let invoice_count = selected.len();
        let gross_total = selected
            .iter()
            .map(|invoice| invoice.gross_obligation())
            .sum();
        let receipt_total = selected
            .iter()
            .map(|invoice| invoice.receipts_applied)
            .sum();
        let memo_total = selected
            .iter()
            .map(|invoice| invoice.credit_memos_applied - invoice.debit_memos_applied)
            .sum();
        let accounting_open = selected
            .iter()
            .map(|invoice| invoice.accounting_outstanding())
            .sum();
        let presentation_open = selected
            .iter()
            .map(|invoice| invoice.presentation_outstanding())
            .sum();
        let invoices = selected.iter().map(|invoice| invoice.id.clone()).collect();
        Self {
            party,
            invoice_count,
            gross_total,
            receipt_total,
            memo_total,
            accounting_open,
            presentation_open,
            invoices,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExposureReport {
    pub accounting_open: Money,
    pub presentation_open: Money,
    pub active_claims: Money,
    pub frozen_claims: Money,
    pub receipts_posted: Money,
    pub invoice_status_counts: BTreeMap<String, usize>,
}

impl ExposureReport {
    pub fn build(
        invoices: &InvoiceBook,
        receipts: &ReceiptBook,
        settlement: &SettlementBook,
    ) -> Self {
        let mut invoice_status_counts = BTreeMap::new();
        for invoice in invoices.values() {
            *invoice_status_counts
                .entry(format!("{:?}", invoice.status))
                .or_insert(0) += 1;
        }

        Self {
            accounting_open: invoices.open_accounting_total(),
            presentation_open: invoices.open_presentation_total(),
            active_claims: settlement.active_claim_total(),
            frozen_claims: settlement.frozen_total(),
            receipts_posted: receipts.values().map(|receipt| receipt.amount).sum(),
            invoice_status_counts,
        }
    }
}
