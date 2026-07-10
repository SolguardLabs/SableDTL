#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

mod adjustments;
mod amount;
mod analytics;
mod close;
mod codec;
mod error;
mod fixtures;
mod ids;
mod invoice;
mod ledger;
mod party;
mod policy;
mod receipts;
mod runtime;
mod settlement;

pub use adjustments::{AdjustmentBook, AdjustmentPolicy, AdjustmentReason, ManualAdjustment};
pub use amount::{Allocation, Bps, Money, Ratio};
pub use analytics::{AgingBand, AgingReport, CounterpartyStatement, ExposureReport};
pub use close::{CloseBatch, CloseStatus, Period, ReconciliationReport};
pub use codec::{Digest, canonical_digest, canonical_json};
pub use error::{SableError, SableResult};
pub use fixtures::{
    AccountMap, ReferenceRecord, SableState, SeedParties, merchant_skus, processor_rules,
    reference_records, seeded_state, treasury_controls,
};
pub use ids::{
    AccountId, AdjustmentId, BatchId, InvoiceId, PartyId, ReceiptId, SettlementId, TxId,
};
pub use invoice::{Invoice, InvoiceBook, InvoiceStatus, LineItem, PaymentTerms};
pub use ledger::{AccountClass, ChartAccount, JournalEntry, JournalLine, Ledger, TrialBalance};
pub use party::{BankAccount, Counterparty, CounterpartyBook, CounterpartyRole};
pub use policy::{BatchPolicy, ClosePolicy, PolicySet, RiskDecision};
pub use receipts::{PaymentChannel, ReceiptBook, ReceiptStatus, SettlementReceipt};
pub use runtime::ScenarioReport;
pub use settlement::{ClaimStatus, SettlementBook, SettlementClaim, SettlementObligation};

fn main() {
    if let Err(error) = runtime::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
