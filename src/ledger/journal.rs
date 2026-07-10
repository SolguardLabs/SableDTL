use serde::{Deserialize, Serialize};

use crate::amount::Money;
use crate::ids::{AccountId, BatchId, InvoiceId, ReceiptId, TxId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalLine {
    pub account: AccountId,
    pub debit: Money,
    pub credit: Money,
    pub memo: String,
}

impl JournalLine {
    pub fn debit(account: AccountId, amount: Money, memo: impl Into<String>) -> Self {
        Self {
            account,
            debit: amount,
            credit: Money::ZERO,
            memo: memo.into(),
        }
    }

    pub fn credit(account: AccountId, amount: Money, memo: impl Into<String>) -> Self {
        Self {
            account,
            debit: Money::ZERO,
            credit: amount,
            memo: memo.into(),
        }
    }

    pub fn signed_effect(&self) -> Money {
        self.debit - self.credit
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub tx_id: TxId,
    pub day: u32,
    pub batch_id: Option<BatchId>,
    pub invoice_id: Option<InvoiceId>,
    pub receipt_id: Option<ReceiptId>,
    pub source: String,
    pub memo: String,
    pub lines: Vec<JournalLine>,
}

impl JournalEntry {
    pub fn new(tx_id: TxId, day: u32, source: impl Into<String>, memo: impl Into<String>) -> Self {
        Self {
            tx_id,
            day,
            batch_id: None,
            invoice_id: None,
            receipt_id: None,
            source: source.into(),
            memo: memo.into(),
            lines: Vec::new(),
        }
    }

    pub fn for_invoice(mut self, invoice_id: InvoiceId) -> Self {
        self.invoice_id = Some(invoice_id);
        self
    }

    pub fn for_receipt(mut self, receipt_id: ReceiptId) -> Self {
        self.receipt_id = Some(receipt_id);
        self
    }

    pub fn in_batch(mut self, batch_id: BatchId) -> Self {
        self.batch_id = Some(batch_id);
        self
    }

    pub fn with_line(mut self, line: JournalLine) -> Self {
        self.lines.push(line);
        self
    }

    pub fn total_debits(&self) -> Money {
        self.lines.iter().map(|line| line.debit).sum()
    }

    pub fn total_credits(&self) -> Money {
        self.lines.iter().map(|line| line.credit).sum()
    }
}
