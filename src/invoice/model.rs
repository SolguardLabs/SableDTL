use serde::{Deserialize, Serialize};

use crate::amount::{Bps, Money};
use crate::error::{SableError, SableResult};
use crate::ids::{InvoiceId, PartyId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PaymentTerms {
    pub net_days: u32,
    pub early_discount_bps: Bps,
    pub late_fee_bps: Bps,
    pub grace_days: u32,
}

impl PaymentTerms {
    pub fn net_30() -> Self {
        Self {
            net_days: 30,
            early_discount_bps: Bps::ZERO,
            late_fee_bps: Bps::ZERO,
            grace_days: 3,
        }
    }

    pub fn with_discount(mut self, discount_bps: Bps) -> Self {
        self.early_discount_bps = discount_bps;
        self
    }

    pub fn with_late_fee(mut self, late_fee_bps: Bps) -> Self {
        self.late_fee_bps = late_fee_bps;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LineItem {
    pub sku: String,
    pub description: String,
    pub quantity: i64,
    pub unit_price: Money,
    pub tax_bps: Bps,
}

impl LineItem {
    pub fn new(
        sku: impl Into<String>,
        description: impl Into<String>,
        quantity: i64,
        unit_price: Money,
        tax_bps: Bps,
    ) -> Self {
        Self {
            sku: sku.into(),
            description: description.into(),
            quantity,
            unit_price,
            tax_bps,
        }
    }

    pub fn net_amount(&self) -> SableResult<Money> {
        self.unit_price.checked_mul_i64(self.quantity)
    }

    pub fn tax_amount(&self) -> SableResult<Money> {
        self.net_amount()?.apply_bps(self.tax_bps)
    }

    pub fn gross_amount(&self) -> SableResult<Money> {
        self.net_amount()?.checked_add(self.tax_amount()?)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Issued,
    PartiallyCollected,
    Paid,
    Closed,
    Voided,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
    pub id: InvoiceId,
    pub merchant: PartyId,
    pub buyer: PartyId,
    pub external_ref: String,
    pub issue_day: u32,
    pub due_day: u32,
    pub terms: PaymentTerms,
    pub line_items: Vec<LineItem>,
    pub subtotal: Money,
    pub tax: Money,
    pub total: Money,
    pub receipts_applied: Money,
    pub credit_memos_applied: Money,
    pub debit_memos_applied: Money,
    pub status: InvoiceStatus,
    pub closed_day: Option<u32>,
    pub tags: Vec<String>,
}

impl Invoice {
    pub fn new(
        id: InvoiceId,
        merchant: PartyId,
        buyer: PartyId,
        external_ref: impl Into<String>,
        issue_day: u32,
        terms: PaymentTerms,
        line_items: Vec<LineItem>,
    ) -> SableResult<Self> {
        let subtotal: Money = line_items
            .iter()
            .map(LineItem::net_amount)
            .collect::<SableResult<Vec<_>>>()?
            .into_iter()
            .sum();
        let tax: Money = line_items
            .iter()
            .map(LineItem::tax_amount)
            .collect::<SableResult<Vec<_>>>()?
            .into_iter()
            .sum();
        let total = subtotal.checked_add(tax)?;
        Ok(Self {
            id,
            merchant,
            buyer,
            external_ref: external_ref.into(),
            issue_day,
            due_day: issue_day + terms.net_days,
            terms,
            line_items,
            subtotal,
            tax,
            total,
            receipts_applied: Money::ZERO,
            credit_memos_applied: Money::ZERO,
            debit_memos_applied: Money::ZERO,
            status: InvoiceStatus::Draft,
            closed_day: None,
            tags: Vec::new(),
        })
    }

    pub fn issue(&mut self) -> SableResult<()> {
        match self.status {
            InvoiceStatus::Draft => {
                self.status = InvoiceStatus::Issued;
                Ok(())
            }
            _ => Err(SableError::InvalidTransition(format!(
                "invoice {} cannot be issued from {:?}",
                self.id, self.status
            ))),
        }
    }

    pub fn apply_receipt(&mut self, amount: Money) -> SableResult<()> {
        if amount.cents() <= 0 {
            return Err(SableError::InvalidTransition(
                "receipt amount must be positive".to_string(),
            ));
        }
        if matches!(self.status, InvoiceStatus::Voided | InvoiceStatus::Closed) {
            return Err(SableError::InvalidTransition(format!(
                "invoice {} does not accept receipts",
                self.id
            )));
        }
        self.receipts_applied = self.receipts_applied.checked_add(amount)?;
        self.refresh_status();
        Ok(())
    }

    pub fn apply_credit_memo(&mut self, amount: Money) -> SableResult<()> {
        if amount.cents() <= 0 {
            return Err(SableError::InvalidTransition(
                "memo amount must be positive".to_string(),
            ));
        }
        if matches!(self.status, InvoiceStatus::Voided | InvoiceStatus::Closed) {
            return Err(SableError::InvalidTransition(format!(
                "invoice {} does not accept memo entries",
                self.id
            )));
        }
        self.credit_memos_applied = self.credit_memos_applied.checked_add(amount)?;
        self.refresh_status();
        Ok(())
    }

    pub fn apply_debit_memo(&mut self, amount: Money) -> SableResult<()> {
        if amount.cents() <= 0 {
            return Err(SableError::InvalidTransition(
                "memo amount must be positive".to_string(),
            ));
        }
        if matches!(self.status, InvoiceStatus::Voided | InvoiceStatus::Closed) {
            return Err(SableError::InvalidTransition(format!(
                "invoice {} does not accept memo entries",
                self.id
            )));
        }
        self.debit_memos_applied = self.debit_memos_applied.checked_add(amount)?;
        self.refresh_status();
        Ok(())
    }

    pub fn close(&mut self, day: u32) -> SableResult<()> {
        if self.accounting_outstanding().cents() > 0 {
            return Err(SableError::InvalidTransition(format!(
                "invoice {} has an open accounting balance",
                self.id
            )));
        }
        self.status = InvoiceStatus::Closed;
        self.closed_day = Some(day);
        Ok(())
    }

    pub fn void(&mut self, day: u32) -> SableResult<()> {
        if self.receipts_applied.cents() != 0 {
            return Err(SableError::InvalidTransition(format!(
                "invoice {} already has receipts",
                self.id
            )));
        }
        self.status = InvoiceStatus::Voided;
        self.closed_day = Some(day);
        Ok(())
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn gross_obligation(&self) -> Money {
        self.total + self.debit_memos_applied
    }

    pub fn accounting_outstanding(&self) -> Money {
        self.gross_obligation()
            .saturating_sub_floor_zero(self.receipts_applied + self.credit_memos_applied)
    }

    pub fn presentation_outstanding(&self) -> Money {
        self.gross_obligation()
            .saturating_sub_floor_zero(self.receipts_applied)
    }

    pub fn has_accounting_balance(&self) -> bool {
        self.accounting_outstanding().cents() > 0
    }

    pub fn has_presentation_balance(&self) -> bool {
        self.presentation_outstanding().cents() > 0
    }

    pub fn is_late(&self, day: u32) -> bool {
        day > self.due_day + self.terms.grace_days && self.has_accounting_balance()
    }

    pub fn days_past_due(&self, day: u32) -> u32 {
        day.saturating_sub(self.due_day)
    }

    pub fn effective_collected_ratio(&self) -> SableResult<Bps> {
        if self.gross_obligation().is_zero() {
            return Ok(Bps::FULL);
        }
        let numerator = (self.receipts_applied.cents() as i128)
            .checked_mul(10_000)
            .ok_or(SableError::ArithmeticOverflow)?;
        let value = numerator / self.gross_obligation().cents() as i128;
        Bps::new(u16::try_from(value.clamp(0, 10_000)).unwrap_or(10_000))
    }

    fn refresh_status(&mut self) {
        if matches!(
            self.status,
            InvoiceStatus::Draft | InvoiceStatus::Voided | InvoiceStatus::Closed
        ) {
            return;
        }

        let outstanding = self.accounting_outstanding();
        self.status = if outstanding.is_zero() {
            InvoiceStatus::Paid
        } else if self.receipts_applied.is_positive() || self.credit_memos_applied.is_positive() {
            InvoiceStatus::PartiallyCollected
        } else {
            InvoiceStatus::Issued
        };
    }
}
