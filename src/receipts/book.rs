use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::{SableError, SableResult};
use crate::ids::{InvoiceId, ReceiptId};
use crate::receipts::{ReceiptStatus, SettlementReceipt};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReceiptBook {
    receipts: BTreeMap<ReceiptId, SettlementReceipt>,
}

impl ReceiptBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, receipt: SettlementReceipt) -> SableResult<()> {
        if self.receipts.contains_key(&receipt.id) {
            return Err(SableError::DuplicateId(receipt.id.to_string()));
        }
        self.receipts.insert(receipt.id.clone(), receipt);
        Ok(())
    }

    pub fn get(&self, id: &ReceiptId) -> SableResult<&SettlementReceipt> {
        self.receipts
            .get(id)
            .ok_or_else(|| SableError::ReceiptNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &ReceiptId) -> SableResult<&mut SettlementReceipt> {
        self.receipts
            .get_mut(id)
            .ok_or_else(|| SableError::ReceiptNotFound(id.clone()))
    }

    pub fn mark_matched(&mut self, id: &ReceiptId) -> SableResult<()> {
        self.get_mut(id)?.mark_matched();
        Ok(())
    }

    pub fn by_invoice(&self, invoice_id: &InvoiceId) -> Vec<&SettlementReceipt> {
        self.receipts
            .values()
            .filter(|receipt| &receipt.invoice_id == invoice_id)
            .collect()
    }

    pub fn posted_by_invoice(&self, invoice_id: &InvoiceId) -> Vec<&SettlementReceipt> {
        self.receipts
            .values()
            .filter(|receipt| {
                &receipt.invoice_id == invoice_id && receipt.status == ReceiptStatus::Posted
            })
            .collect()
    }

    pub fn values(&self) -> impl Iterator<Item = &SettlementReceipt> {
        self.receipts.values()
    }

    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}
