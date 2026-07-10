use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Money;
use crate::error::{SableError, SableResult};
use crate::ids::{AccountId, BatchId, InvoiceId, SettlementId};
use crate::invoice::InvoiceBook;
use crate::ledger::Ledger;
use crate::settlement::{ClaimStatus, SettlementClaim, SettlementObligation};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SettlementBook {
    claims: BTreeMap<SettlementId, SettlementClaim>,
    obligations: BTreeMap<InvoiceId, SettlementObligation>,
    next_claim: u64,
}

impl SettlementBook {
    pub fn new() -> Self {
        Self {
            claims: BTreeMap::new(),
            obligations: BTreeMap::new(),
            next_claim: 1,
        }
    }

    pub fn refresh(&mut self, invoices: &InvoiceBook) {
        self.obligations.clear();
        for invoice in invoices.values() {
            if let Some(obligation) = SettlementObligation::from_invoice(invoice) {
                self.obligations
                    .insert(obligation.invoice_id.clone(), obligation);
            }
        }
    }

    pub fn prepare_due_claims(
        &mut self,
        day: u32,
        batch_id: BatchId,
    ) -> SableResult<Vec<SettlementId>> {
        let mut due: Vec<_> = self
            .obligations
            .values()
            .filter(|obligation| obligation.is_due(day))
            .cloned()
            .collect();
        due.sort_by_key(|obligation| -obligation.priority_score(day));

        let mut ids = Vec::new();
        for obligation in due {
            if self.has_active_claim_for(&obligation.invoice_id) {
                continue;
            }
            let id = SettlementId::generated(self.next_claim);
            self.next_claim += 1;
            let claim = SettlementClaim::new(id.clone(), &obligation, day, batch_id.clone());
            self.claims.insert(id.clone(), claim);
            ids.push(id);
        }
        Ok(ids)
    }

    pub fn post_claims(
        &mut self,
        claim_ids: &[SettlementId],
        day: u32,
        ledger: &mut Ledger,
        receivable_account: AccountId,
        clearing_account: AccountId,
    ) -> SableResult<()> {
        for claim_id in claim_ids {
            let claim = self
                .claims
                .get_mut(claim_id)
                .ok_or_else(|| SableError::SettlementNotFound(claim_id.clone()))?;
            if claim.status != ClaimStatus::Prepared {
                continue;
            }
            let tx_id = ledger.post_settlement_claim(
                day,
                claim.invoice_id.clone(),
                claim.batch_id.clone(),
                receivable_account.clone(),
                clearing_account.clone(),
                claim.amount,
            )?;
            claim.mark_posted(tx_id);
        }
        Ok(())
    }

    pub fn settle_claim(&mut self, id: &SettlementId) -> SableResult<()> {
        self.get_mut(id)?.settle();
        Ok(())
    }

    pub fn freeze_claim(&mut self, id: &SettlementId) -> SableResult<()> {
        self.get_mut(id)?.freeze();
        Ok(())
    }

    pub fn cancel_claim(&mut self, id: &SettlementId) -> SableResult<()> {
        self.get_mut(id)?.cancel();
        Ok(())
    }

    pub fn get(&self, id: &SettlementId) -> SableResult<&SettlementClaim> {
        self.claims
            .get(id)
            .ok_or_else(|| SableError::SettlementNotFound(id.clone()))
    }

    pub fn get_mut(&mut self, id: &SettlementId) -> SableResult<&mut SettlementClaim> {
        self.claims
            .get_mut(id)
            .ok_or_else(|| SableError::SettlementNotFound(id.clone()))
    }

    pub fn obligations(&self) -> impl Iterator<Item = &SettlementObligation> {
        self.obligations.values()
    }

    pub fn claims(&self) -> impl Iterator<Item = &SettlementClaim> {
        self.claims.values()
    }

    pub fn active_claim_total(&self) -> Money {
        self.claims
            .values()
            .filter(|claim| matches!(claim.status, ClaimStatus::Prepared | ClaimStatus::Posted))
            .map(|claim| claim.amount)
            .sum()
    }

    pub fn frozen_total(&self) -> Money {
        self.claims
            .values()
            .filter(|claim| claim.status == ClaimStatus::Frozen)
            .map(|claim| claim.amount)
            .sum()
    }

    pub fn obligation_total(&self) -> Money {
        self.obligations
            .values()
            .map(|obligation| obligation.presentable_amount)
            .sum()
    }

    pub fn posted_count(&self) -> usize {
        self.claims
            .values()
            .filter(|claim| claim.status == ClaimStatus::Posted)
            .count()
    }

    pub fn has_active_claim_for(&self, invoice_id: &InvoiceId) -> bool {
        self.claims.values().any(|claim| {
            &claim.invoice_id == invoice_id
                && matches!(claim.status, ClaimStatus::Prepared | ClaimStatus::Posted)
        })
    }
}
