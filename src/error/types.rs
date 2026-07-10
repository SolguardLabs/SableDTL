use thiserror::Error;

use crate::ids::{AccountId, AdjustmentId, InvoiceId, PartyId, ReceiptId, SettlementId};

pub type SableResult<T> = Result<T, SableError>;

#[derive(Debug, Error)]
pub enum SableError {
    #[error("arithmetic overflow")]
    ArithmeticOverflow,
    #[error("invalid basis points value {0}")]
    InvalidBasisPoints(u16),
    #[error("invalid ratio")]
    InvalidRatio,
    #[error("counterparty not found: {0}")]
    CounterpartyNotFound(PartyId),
    #[error("account not found: {0}")]
    AccountNotFound(AccountId),
    #[error("invoice not found: {0}")]
    InvoiceNotFound(InvoiceId),
    #[error("receipt not found: {0}")]
    ReceiptNotFound(ReceiptId),
    #[error("adjustment not found: {0}")]
    AdjustmentNotFound(AdjustmentId),
    #[error("settlement not found: {0}")]
    SettlementNotFound(SettlementId),
    #[error("duplicate id: {0}")]
    DuplicateId(String),
    #[error("invalid state transition: {0}")]
    InvalidTransition(String),
    #[error("journal entry is not balanced: debits={debits}, credits={credits}")]
    UnbalancedJournal { debits: i64, credits: i64 },
    #[error("policy rejected operation: {0}")]
    PolicyRejected(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("unknown scenario: {0}")]
    UnknownScenario(String),
}
