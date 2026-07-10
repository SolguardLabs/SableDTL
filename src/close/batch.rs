use serde::{Deserialize, Serialize};

use crate::amount::Money;
use crate::analytics::{AgingReport, ExposureReport};
use crate::codec::{Digest, canonical_digest};
use crate::error::{SableError, SableResult};
use crate::ids::BatchId;
use crate::invoice::InvoiceBook;
use crate::ledger::{Ledger, TrialBalance};
use crate::policy::ClosePolicy;
use crate::receipts::ReceiptBook;
use crate::settlement::SettlementBook;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Period {
    pub year: u16,
    pub month: u8,
    pub start_day: u32,
    pub end_day: u32,
}

impl Period {
    pub fn new(year: u16, month: u8, start_day: u32, end_day: u32) -> Self {
        Self {
            year,
            month,
            start_day,
            end_day,
        }
    }

    pub fn label(&self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }

    pub fn contains(&self, day: u32) -> bool {
        day >= self.start_day && day <= self.end_day
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CloseStatus {
    Open,
    Prepared,
    Closed,
    Reopened,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReconciliationReport {
    pub period: Period,
    pub invoice_count: usize,
    pub receipt_count: usize,
    pub gross_total: Money,
    pub receipt_total: Money,
    pub memo_total: Money,
    pub accounting_open: Money,
    pub presentation_open: Money,
    pub settlement_obligations: Money,
    pub aging: AgingReport,
    pub exposure: ExposureReport,
    pub trial_balance: TrialBalance,
}

impl ReconciliationReport {
    pub fn build(
        period: Period,
        invoices: &InvoiceBook,
        receipts: &ReceiptBook,
        settlement: &SettlementBook,
        ledger: &Ledger,
    ) -> Self {
        let aging = AgingReport::build(invoices, period.end_day);
        let exposure = ExposureReport::build(invoices, receipts, settlement);
        Self {
            period,
            invoice_count: invoices.len(),
            receipt_count: receipts.len(),
            gross_total: invoices
                .values()
                .map(|invoice| invoice.gross_obligation())
                .sum(),
            receipt_total: invoices.collected_total(),
            memo_total: invoices.memo_total(),
            accounting_open: invoices.open_accounting_total(),
            presentation_open: invoices.open_presentation_total(),
            settlement_obligations: settlement.obligation_total(),
            aging,
            exposure,
            trial_balance: ledger.trial_balance(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CloseBatch {
    pub id: BatchId,
    pub period: Period,
    pub status: CloseStatus,
    pub prepared_day: u32,
    pub closed_day: Option<u32>,
    pub report: ReconciliationReport,
    pub digest: Digest,
}

impl CloseBatch {
    pub fn prepare(
        id: BatchId,
        period: Period,
        prepared_day: u32,
        invoices: &InvoiceBook,
        receipts: &ReceiptBook,
        settlement: &SettlementBook,
        ledger: &Ledger,
    ) -> SableResult<Self> {
        let report =
            ReconciliationReport::build(period.clone(), invoices, receipts, settlement, ledger);
        let digest = canonical_digest(&report)?;
        Ok(Self {
            id,
            period,
            status: CloseStatus::Prepared,
            prepared_day,
            closed_day: None,
            report,
            digest,
        })
    }

    pub fn close(&mut self, policy: &ClosePolicy, day: u32) -> SableResult<()> {
        if policy.require_balanced_trial && !self.report.trial_balance.balanced {
            return Err(SableError::PolicyRejected(
                "trial balance must be balanced".to_string(),
            ));
        }
        self.status = CloseStatus::Closed;
        self.closed_day = Some(day);
        Ok(())
    }
}
