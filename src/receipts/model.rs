use serde::{Deserialize, Serialize};

use crate::amount::Money;
use crate::ids::{InvoiceId, PartyId, ReceiptId, TxId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PaymentChannel {
    Ach,
    Wire,
    CardNetwork,
    StablecoinRail,
    InternalTransfer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReceiptStatus {
    Imported,
    Matched,
    Posted,
    Reversed,
    Suspense,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementReceipt {
    pub id: ReceiptId,
    pub invoice_id: InvoiceId,
    pub payer: PartyId,
    pub payee: PartyId,
    pub channel: PaymentChannel,
    pub bank_ref: String,
    pub amount: Money,
    pub received_day: u32,
    pub status: ReceiptStatus,
    pub journal_tx: Option<TxId>,
}

impl SettlementReceipt {
    pub fn new(
        id: ReceiptId,
        invoice_id: InvoiceId,
        payer: PartyId,
        payee: PartyId,
        channel: PaymentChannel,
        bank_ref: impl Into<String>,
        amount: Money,
        received_day: u32,
    ) -> Self {
        Self {
            id,
            invoice_id,
            payer,
            payee,
            channel,
            bank_ref: bank_ref.into(),
            amount,
            received_day,
            status: ReceiptStatus::Imported,
            journal_tx: None,
        }
    }

    pub fn mark_matched(&mut self) {
        if self.status == ReceiptStatus::Imported {
            self.status = ReceiptStatus::Matched;
        }
    }

    pub fn mark_posted(&mut self, tx_id: TxId) {
        self.status = ReceiptStatus::Posted;
        self.journal_tx = Some(tx_id);
    }

    pub fn reverse(&mut self) {
        self.status = ReceiptStatus::Reversed;
    }
}
