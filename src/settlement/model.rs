use serde::{Deserialize, Serialize};

use crate::amount::Money;
use crate::ids::{BatchId, InvoiceId, PartyId, SettlementId, TxId};
use crate::invoice::{Invoice, InvoiceStatus};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementObligation {
    pub invoice_id: InvoiceId,
    pub buyer: PartyId,
    pub merchant: PartyId,
    pub due_day: u32,
    pub original_amount: Money,
    pub collected_amount: Money,
    pub presentable_amount: Money,
    pub status: InvoiceStatus,
}

impl SettlementObligation {
    pub fn from_invoice(invoice: &Invoice) -> Option<Self> {
        if matches!(invoice.status, InvoiceStatus::Draft | InvoiceStatus::Voided) {
            return None;
        }
        let presentable_amount = invoice.presentation_outstanding();
        if presentable_amount.is_zero() {
            return None;
        }
        Some(Self {
            invoice_id: invoice.id.clone(),
            buyer: invoice.buyer.clone(),
            merchant: invoice.merchant.clone(),
            due_day: invoice.due_day,
            original_amount: invoice.gross_obligation(),
            collected_amount: invoice.receipts_applied,
            presentable_amount,
            status: invoice.status.clone(),
        })
    }

    pub fn is_due(&self, day: u32) -> bool {
        day >= self.due_day
    }

    pub fn priority_score(&self, day: u32) -> i64 {
        let age = day.saturating_sub(self.due_day) as i64;
        let amount_score = self.presentable_amount.cents() / 100;
        age.saturating_mul(10) + amount_score
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimStatus {
    Prepared,
    Posted,
    Settled,
    Frozen,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementClaim {
    pub id: SettlementId,
    pub invoice_id: InvoiceId,
    pub buyer: PartyId,
    pub merchant: PartyId,
    pub amount: Money,
    pub created_day: u32,
    pub batch_id: BatchId,
    pub status: ClaimStatus,
    pub journal_tx: Option<TxId>,
}

impl SettlementClaim {
    pub fn new(
        id: SettlementId,
        obligation: &SettlementObligation,
        created_day: u32,
        batch_id: BatchId,
    ) -> Self {
        Self {
            id,
            invoice_id: obligation.invoice_id.clone(),
            buyer: obligation.buyer.clone(),
            merchant: obligation.merchant.clone(),
            amount: obligation.presentable_amount,
            created_day,
            batch_id,
            status: ClaimStatus::Prepared,
            journal_tx: None,
        }
    }

    pub fn mark_posted(&mut self, tx_id: TxId) {
        self.status = ClaimStatus::Posted;
        self.journal_tx = Some(tx_id);
    }

    pub fn settle(&mut self) {
        self.status = ClaimStatus::Settled;
    }

    pub fn freeze(&mut self) {
        self.status = ClaimStatus::Frozen;
    }

    pub fn cancel(&mut self) {
        self.status = ClaimStatus::Cancelled;
    }
}
