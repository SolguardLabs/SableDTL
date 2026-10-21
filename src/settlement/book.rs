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
        let mut candidate_book = self.clone();
        let mut candidate_ledger = ledger.clone();
        candidate_book.post_claims_inner(
            claim_ids,
            day,
            &mut candidate_ledger,
            receivable_account,
            clearing_account,
        )?;
        *self = candidate_book;
        *ledger = candidate_ledger;
        Ok(())
    }

    fn post_claims_inner(
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

#[cfg(test)]
mod tests {
    use crate::amount::{Bps, Money};
    use crate::fixtures::seeded_state;
    use crate::ids::SettlementId;

    #[test]
    fn posting_batch_rolls_back_when_a_later_claim_is_missing() {
        let mut state = seeded_state().unwrap();
        state
            .issue_standard_invoice(
                state.parties.buyer_alpha.clone(),
                "AR-atomic-1",
                1,
                Money::dollars(100),
                Bps::ZERO,
            )
            .unwrap();
        state.refresh_settlement();
        let batch = state.next_batch_id();
        let mut claims = state.settlement.prepare_due_claims(40, batch).unwrap();
        assert_eq!(claims.len(), 1);
        claims.push(SettlementId::from("set-missing"));
        let journal_len = state.ledger.journal().len();

        assert!(
            state
                .settlement
                .post_claims(
                    &claims,
                    40,
                    &mut state.ledger,
                    state.accounts.receivable,
                    state.accounts.clearing,
                )
                .is_err()
        );
        assert_eq!(state.ledger.journal().len(), journal_len);
        assert_eq!(state.settlement.posted_count(), 0);
        assert_eq!(
            state.settlement.get(&claims[0]).unwrap().status,
            crate::settlement::ClaimStatus::Prepared
        );
    }
}
