use serde::Serialize;
use std::collections::BTreeMap;

use crate::adjustments::AdjustmentReason;
use crate::amount::{Bps, Money};
use crate::close::{CloseBatch, Period};
use crate::codec::canonical_digest;
use crate::error::{SableError, SableResult};
use crate::fixtures::seeded_state;
use crate::ids::{InvoiceId, PartyId};
use crate::invoice::InvoiceStatus;
use crate::ledger::TrialBalance;
use crate::receipts::PaymentChannel;
use crate::settlement::ClaimStatus;

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioReport {
    pub scenario: String,
    pub day: u32,
    pub digest: String,
    pub invoices: InvoiceMetrics,
    pub ledger: LedgerMetrics,
    pub settlement: SettlementMetrics,
    pub close: Option<CloseMetrics>,
    pub invoice_rows: Vec<InvoiceRow>,
    pub claim_rows: Vec<ClaimRow>,
    pub events: Vec<String>,
    pub conservation_ok: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvoiceMetrics {
    pub count: usize,
    pub accounting_open: Money,
    pub presentation_open: Money,
    pub collected: Money,
    pub memos: Money,
    pub statuses: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LedgerMetrics {
    pub accounts: usize,
    pub entries: usize,
    pub balanced: bool,
    pub total_debits: Money,
    pub total_credits: Money,
}

#[derive(Clone, Debug, Serialize)]
pub struct SettlementMetrics {
    pub obligations: usize,
    pub obligation_total: Money,
    pub claims: usize,
    pub posted_claims: usize,
    pub active_claim_total: Money,
    pub frozen_claim_total: Money,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloseMetrics {
    pub id: String,
    pub period: String,
    pub status: String,
    pub accounting_open: Money,
    pub presentation_open: Money,
    pub settlement_obligations: Money,
    pub digest: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvoiceRow {
    pub id: InvoiceId,
    pub buyer: PartyId,
    pub status: InvoiceStatus,
    pub total: Money,
    pub receipts: Money,
    pub memos: Money,
    pub accounting_open: Money,
    pub presentation_open: Money,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClaimRow {
    pub id: String,
    pub invoice_id: InvoiceId,
    pub amount: Money,
    pub status: ClaimStatus,
}

#[derive(Clone, Debug, Serialize)]
struct ReportFingerprint {
    scenario: String,
    day: u32,
    invoices: InvoiceMetrics,
    ledger: LedgerMetrics,
    settlement: SettlementMetrics,
    close: Option<CloseMetrics>,
    invoice_rows: Vec<InvoiceRow>,
    claim_rows: Vec<ClaimRow>,
    events: Vec<String>,
    conservation_ok: bool,
}

pub fn run_named(name: &str) -> SableResult<ScenarioReport> {
    match name {
        "valid" => valid_payments(),
        "adjustments" => small_adjustments(),
        "monthly-close" => monthly_close(),
        "final" => final_reconciliation(),
        "receipts" => receipt_imports(),
        other => Err(SableError::UnknownScenario(other.to_string())),
    }
}

fn valid_payments() -> SableResult<ScenarioReport> {
    let mut state = seeded_state()?;
    let invoice_a = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0001",
        1,
        Money::dollars(24_000),
        Bps::ZERO,
    )?;
    let invoice_b = state.issue_standard_invoice(
        state.parties.buyer_beta.clone(),
        "AR-2026-0002",
        2,
        Money::dollars(13_500),
        Bps::ZERO,
    )?;
    state.record_receipt(
        &invoice_a,
        8,
        Money::dollars(24_000),
        PaymentChannel::Ach,
        "ACH-9120",
    )?;
    state.record_receipt(
        &invoice_b,
        10,
        Money::dollars(13_500),
        PaymentChannel::Wire,
        "WIRE-4450",
    )?;
    state.refresh_settlement();
    build_report(
        state,
        "valid",
        10,
        None,
        vec!["posted two receipts".to_string()],
    )
}

fn small_adjustments() -> SableResult<ScenarioReport> {
    let mut state = seeded_state()?;
    let invoice = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0100",
        1,
        Money::dollars(100_000),
        Bps::ZERO,
    )?;
    state.record_receipt(
        &invoice,
        11,
        Money::dollars(97_500),
        PaymentChannel::Ach,
        "ACH-7741",
    )?;
    state.record_credit_memo(
        &invoice,
        12,
        Money::dollars(1_250),
        AdjustmentReason::ContractRebate,
    )?;
    state.record_credit_memo(
        &invoice,
        13,
        Money::dollars(1_250),
        AdjustmentReason::ProcessorFeeTrueUp,
    )?;
    state.refresh_settlement();
    build_report(
        state,
        "adjustments",
        13,
        None,
        vec!["posted receipt and two approved memos".to_string()],
    )
}

fn monthly_close() -> SableResult<ScenarioReport> {
    let mut state = seeded_state()?;
    let invoice_a = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0200",
        1,
        Money::dollars(42_000),
        Bps::ZERO,
    )?;
    let invoice_b = state.issue_standard_invoice(
        state.parties.buyer_beta.clone(),
        "AR-2026-0201",
        4,
        Money::dollars(18_000),
        Bps::ZERO,
    )?;
    let invoice_c = state.issue_standard_invoice(
        state.parties.buyer_gamma.clone(),
        "AR-2026-0202",
        7,
        Money::dollars(11_200),
        Bps::ZERO,
    )?;
    state.record_receipt(
        &invoice_a,
        12,
        Money::dollars(42_000),
        PaymentChannel::Wire,
        "WIRE-7652",
    )?;
    state.record_receipt(
        &invoice_b,
        20,
        Money::dollars(17_550),
        PaymentChannel::Ach,
        "ACH-1099",
    )?;
    state.record_credit_memo(
        &invoice_b,
        22,
        Money::dollars(450),
        AdjustmentReason::FreightVariance,
    )?;
    state.record_receipt(
        &invoice_c,
        26,
        Money::dollars(5_600),
        PaymentChannel::CardNetwork,
        "CARD-BATCH-31",
    )?;
    let closed = state.close_eligible_invoices(31)?;
    let mut close = state.prepare_close(Period::new(2026, 1, 1, 31), 31)?;
    close.close(&state.policies.close, 31)?;
    build_report(
        state,
        "monthly-close",
        31,
        Some(close),
        vec![format!("closed {closed} eligible invoices")],
    )
}

fn final_reconciliation() -> SableResult<ScenarioReport> {
    let mut state = seeded_state()?;
    let invoice_a = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0300",
        1,
        Money::dollars(72_000),
        Bps::ZERO,
    )?;
    let invoice_b = state.issue_standard_invoice(
        state.parties.buyer_beta.clone(),
        "AR-2026-0301",
        2,
        Money::dollars(54_000),
        Bps::ZERO,
    )?;
    let invoice_c = state.issue_standard_invoice(
        state.parties.buyer_gamma.clone(),
        "AR-2026-0302",
        6,
        Money::dollars(21_500),
        Bps::ZERO,
    )?;
    state.record_receipt(
        &invoice_a,
        9,
        Money::dollars(72_000),
        PaymentChannel::Wire,
        "WIRE-3090",
    )?;
    state.record_receipt(
        &invoice_b,
        16,
        Money::dollars(51_300),
        PaymentChannel::Ach,
        "ACH-5451",
    )?;
    state.record_credit_memo(
        &invoice_b,
        18,
        Money::dollars(1_350),
        AdjustmentReason::ServiceCredit,
    )?;
    state.record_credit_memo(
        &invoice_b,
        19,
        Money::dollars(1_350),
        AdjustmentReason::ProcessorFeeTrueUp,
    )?;
    state.record_receipt(
        &invoice_c,
        20,
        Money::dollars(8_000),
        PaymentChannel::StablecoinRail,
        "STBL-8801",
    )?;
    let claims = state.prepare_and_post_settlement(40)?;
    let close = state.prepare_close(Period::new(2026, 1, 1, 31), 40)?;
    build_report(
        state,
        "final",
        40,
        Some(close),
        vec![format!("prepared {} settlement claims", claims.len())],
    )
}

fn receipt_imports() -> SableResult<ScenarioReport> {
    let mut state = seeded_state()?;
    let invoice_a = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0400",
        3,
        Money::dollars(16_000),
        Bps::ZERO,
    )?;
    let invoice_b = state.issue_standard_invoice(
        state.parties.buyer_alpha.clone(),
        "AR-2026-0401",
        3,
        Money::dollars(9_500),
        Bps::ZERO,
    )?;
    state.record_receipt(
        &invoice_a,
        5,
        Money::dollars(16_000),
        PaymentChannel::Ach,
        "ACH-2201",
    )?;
    state.record_receipt(
        &invoice_b,
        5,
        Money::dollars(4_750),
        PaymentChannel::Ach,
        "ACH-2202",
    )?;
    state.refresh_settlement();
    build_report(
        state,
        "receipts",
        5,
        None,
        vec!["imported same-day processor file".to_string()],
    )
}

fn build_report(
    state: crate::fixtures::SableState,
    scenario: &str,
    day: u32,
    close: Option<CloseBatch>,
    events: Vec<String>,
) -> SableResult<ScenarioReport> {
    let trial = state.ledger.trial_balance();
    let invoices = invoice_metrics(&state);
    let ledger = ledger_metrics(&state, &trial);
    let settlement = settlement_metrics(&state);
    let close = close.map(close_metrics);
    let invoice_rows = invoice_rows(&state);
    let claim_rows = claim_rows(&state);
    let conservation_ok = ledger.balanced;
    let fingerprint = ReportFingerprint {
        scenario: scenario.to_string(),
        day,
        invoices: invoices.clone(),
        ledger: ledger.clone(),
        settlement: settlement.clone(),
        close: close.clone(),
        invoice_rows: invoice_rows.clone(),
        claim_rows: claim_rows.clone(),
        events: events.clone(),
        conservation_ok,
    };
    let digest = canonical_digest(&fingerprint)?.to_string();
    Ok(ScenarioReport {
        scenario: scenario.to_string(),
        day,
        digest,
        invoices,
        ledger,
        settlement,
        close,
        invoice_rows,
        claim_rows,
        events,
        conservation_ok,
    })
}

fn invoice_metrics(state: &crate::fixtures::SableState) -> InvoiceMetrics {
    let mut statuses = BTreeMap::new();
    for invoice in state.invoices.values() {
        *statuses.entry(format!("{:?}", invoice.status)).or_insert(0) += 1;
    }
    InvoiceMetrics {
        count: state.invoices.len(),
        accounting_open: state.invoices.open_accounting_total(),
        presentation_open: state.invoices.open_presentation_total(),
        collected: state.invoices.collected_total(),
        memos: state.invoices.memo_total(),
        statuses,
    }
}

fn ledger_metrics(state: &crate::fixtures::SableState, trial: &TrialBalance) -> LedgerMetrics {
    LedgerMetrics {
        accounts: state.ledger.account_count(),
        entries: state.ledger.journal().len(),
        balanced: trial.balanced,
        total_debits: trial.total_debits,
        total_credits: trial.total_credits,
    }
}

fn settlement_metrics(state: &crate::fixtures::SableState) -> SettlementMetrics {
    SettlementMetrics {
        obligations: state.settlement.obligations().count(),
        obligation_total: state.settlement.obligation_total(),
        claims: state.settlement.claims().count(),
        posted_claims: state.settlement.posted_count(),
        active_claim_total: state.settlement.active_claim_total(),
        frozen_claim_total: state.settlement.frozen_total(),
    }
}

fn close_metrics(close: CloseBatch) -> CloseMetrics {
    CloseMetrics {
        id: close.id.to_string(),
        period: close.period.label(),
        status: format!("{:?}", close.status),
        accounting_open: close.report.accounting_open,
        presentation_open: close.report.presentation_open,
        settlement_obligations: close.report.settlement_obligations,
        digest: close.digest.to_string(),
    }
}

fn invoice_rows(state: &crate::fixtures::SableState) -> Vec<InvoiceRow> {
    state
        .invoices
        .values()
        .map(|invoice| InvoiceRow {
            id: invoice.id.clone(),
            buyer: invoice.buyer.clone(),
            status: invoice.status.clone(),
            total: invoice.gross_obligation(),
            receipts: invoice.receipts_applied,
            memos: invoice.credit_memos_applied - invoice.debit_memos_applied,
            accounting_open: invoice.accounting_outstanding(),
            presentation_open: invoice.presentation_outstanding(),
        })
        .collect()
}

fn claim_rows(state: &crate::fixtures::SableState) -> Vec<ClaimRow> {
    state
        .settlement
        .claims()
        .map(|claim| ClaimRow {
            id: claim.id.to_string(),
            invoice_id: claim.invoice_id.clone(),
            amount: claim.amount,
            status: claim.status.clone(),
        })
        .collect()
}
