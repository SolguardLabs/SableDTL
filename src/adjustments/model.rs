use serde::{Deserialize, Serialize};

use crate::amount::{Bps, Money};
use crate::error::{SableError, SableResult};
use crate::ids::{AdjustmentId, InvoiceId, PartyId, TxId};
use crate::invoice::Invoice;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AdjustmentReason {
    FreightVariance,
    TaxRounding,
    ServiceCredit,
    ContractRebate,
    ProcessorFeeTrueUp,
    ManualReview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AdjustmentSide {
    CreditMemo,
    DebitMemo,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdjustmentPolicy {
    pub max_single_bps: Bps,
    pub max_period_bps: Bps,
    pub require_dual_approval: bool,
}

impl AdjustmentPolicy {
    pub fn conservative() -> Self {
        Self {
            max_single_bps: Bps::new(250).expect("valid bps"),
            max_period_bps: Bps::new(500).expect("valid bps"),
            require_dual_approval: true,
        }
    }

    pub fn validate(&self, invoice: &Invoice, adjustment: &ManualAdjustment) -> SableResult<()> {
        if adjustment.amount.cents() <= 0 {
            return Err(SableError::PolicyRejected(
                "adjustment amount must be positive".to_string(),
            ));
        }
        let max_amount = invoice.gross_obligation().apply_bps(self.max_single_bps)?;
        if adjustment.amount > max_amount {
            return Err(SableError::PolicyRejected(format!(
                "adjustment {} exceeds single-entry policy",
                adjustment.id
            )));
        }
        if self.require_dual_approval && adjustment.approved_by.is_none() {
            return Err(SableError::PolicyRejected(format!(
                "adjustment {} requires approval",
                adjustment.id
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ManualAdjustment {
    pub id: AdjustmentId,
    pub invoice_id: InvoiceId,
    pub side: AdjustmentSide,
    pub reason: AdjustmentReason,
    pub amount: Money,
    pub created_by: PartyId,
    pub approved_by: Option<PartyId>,
    pub day: u32,
    pub memo: String,
    pub journal_tx: Option<TxId>,
}

impl ManualAdjustment {
    pub fn credit(
        id: AdjustmentId,
        invoice_id: InvoiceId,
        reason: AdjustmentReason,
        amount: Money,
        created_by: PartyId,
        day: u32,
    ) -> Self {
        Self {
            id,
            invoice_id,
            side: AdjustmentSide::CreditMemo,
            reason,
            amount,
            created_by,
            approved_by: None,
            day,
            memo: String::new(),
            journal_tx: None,
        }
    }

    pub fn debit(
        id: AdjustmentId,
        invoice_id: InvoiceId,
        reason: AdjustmentReason,
        amount: Money,
        created_by: PartyId,
        day: u32,
    ) -> Self {
        Self {
            id,
            invoice_id,
            side: AdjustmentSide::DebitMemo,
            reason,
            amount,
            created_by,
            approved_by: None,
            day,
            memo: String::new(),
            journal_tx: None,
        }
    }

    pub fn approve(mut self, approver: PartyId) -> Self {
        self.approved_by = Some(approver);
        self
    }

    pub fn with_memo(mut self, memo: impl Into<String>) -> Self {
        self.memo = memo.into();
        self
    }

    pub fn mark_posted(&mut self, tx_id: TxId) {
        self.journal_tx = Some(tx_id);
    }
}
