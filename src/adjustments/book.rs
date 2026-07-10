use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::adjustments::{AdjustmentPolicy, AdjustmentSide, ManualAdjustment};
use crate::error::{SableError, SableResult};
use crate::ids::{AccountId, AdjustmentId};
use crate::invoice::InvoiceBook;
use crate::ledger::Ledger;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdjustmentBook {
    adjustments: BTreeMap<AdjustmentId, ManualAdjustment>,
}

impl AdjustmentBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, adjustment: ManualAdjustment) -> SableResult<()> {
        if self.adjustments.contains_key(&adjustment.id) {
            return Err(SableError::DuplicateId(adjustment.id.to_string()));
        }
        self.adjustments.insert(adjustment.id.clone(), adjustment);
        Ok(())
    }

    pub fn get(&self, id: &AdjustmentId) -> SableResult<&ManualAdjustment> {
        self.adjustments
            .get(id)
            .ok_or_else(|| SableError::AdjustmentNotFound(id.clone()))
    }

    pub fn values(&self) -> impl Iterator<Item = &ManualAdjustment> {
        self.adjustments.values()
    }

    pub fn apply(
        &mut self,
        mut adjustment: ManualAdjustment,
        policy: &AdjustmentPolicy,
        invoices: &mut InvoiceBook,
        ledger: &mut Ledger,
        receivable_account: AccountId,
        revenue_account: AccountId,
        allowance_account: AccountId,
    ) -> SableResult<()> {
        let invoice = invoices.get(&adjustment.invoice_id)?.clone();
        policy.validate(&invoice, &adjustment)?;
        let tx_id = match adjustment.side {
            AdjustmentSide::CreditMemo => {
                invoices.apply_credit_memo(&adjustment.invoice_id, adjustment.amount)?;
                ledger.post_credit_memo(
                    adjustment.day,
                    adjustment.invoice_id.clone(),
                    receivable_account,
                    allowance_account,
                    adjustment.amount,
                )?
            }
            AdjustmentSide::DebitMemo => {
                invoices.apply_debit_memo(&adjustment.invoice_id, adjustment.amount)?;
                ledger.post_debit_memo(
                    adjustment.day,
                    adjustment.invoice_id.clone(),
                    receivable_account,
                    revenue_account,
                    adjustment.amount,
                )?
            }
        };
        adjustment.mark_posted(tx_id);
        self.insert(adjustment)
    }

    pub fn len(&self) -> usize {
        self.adjustments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adjustments.is_empty()
    }
}
