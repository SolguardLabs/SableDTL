use serde::{Deserialize, Serialize};

use crate::amount::{Bps, Money};
use crate::error::{SableError, SableResult};
use crate::settlement::SettlementBook;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RiskDecision {
    Accepted,
    ReviewRequired(String),
    Rejected(String),
}

impl RiskDecision {
    pub fn is_accepted(&self) -> bool {
        matches!(self, RiskDecision::Accepted)
    }

    pub fn into_result(self) -> SableResult<()> {
        match self {
            RiskDecision::Accepted => Ok(()),
            RiskDecision::ReviewRequired(reason) | RiskDecision::Rejected(reason) => {
                Err(SableError::PolicyRejected(reason))
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchPolicy {
    pub max_claims: usize,
    pub max_batch_amount: Money,
    pub review_threshold: Money,
}

impl BatchPolicy {
    pub fn standard() -> Self {
        Self {
            max_claims: 250,
            max_batch_amount: Money::dollars(5_000_000),
            review_threshold: Money::dollars(250_000),
        }
    }

    pub fn evaluate(&self, settlement: &SettlementBook) -> RiskDecision {
        let count = settlement.posted_count();
        let amount = settlement.active_claim_total();
        if count > self.max_claims {
            return RiskDecision::Rejected(format!("claim count {count} exceeds policy"));
        }
        if amount > self.max_batch_amount {
            return RiskDecision::Rejected(format!("claim amount {amount} exceeds policy"));
        }
        if amount > self.review_threshold {
            return RiskDecision::ReviewRequired(format!("claim amount {amount} requires review"));
        }
        RiskDecision::Accepted
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClosePolicy {
    pub allowed_receivable_drift_bps: Bps,
    pub require_balanced_trial: bool,
    pub allow_open_dispute_freeze: bool,
}

impl ClosePolicy {
    pub fn standard() -> Self {
        Self {
            allowed_receivable_drift_bps: Bps::new(10).expect("valid bps"),
            require_balanced_trial: true,
            allow_open_dispute_freeze: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PolicySet {
    pub batch: BatchPolicy,
    pub close: ClosePolicy,
}

impl PolicySet {
    pub fn standard() -> Self {
        Self {
            batch: BatchPolicy::standard(),
            close: ClosePolicy::standard(),
        }
    }
}
