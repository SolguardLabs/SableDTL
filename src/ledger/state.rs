use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Money;
use crate::error::{SableError, SableResult};
use crate::ids::{AccountId, BatchId, InvoiceId, ReceiptId, TxId};
use crate::ledger::{AccountClass, ChartAccount, JournalEntry, JournalLine};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrialBalance {
    pub accounts: Vec<TrialBalanceRow>,
    pub total_debits: Money,
    pub total_credits: Money,
    pub balanced: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TrialBalanceRow {
    pub account: AccountId,
    pub code: String,
    pub name: String,
    pub class: AccountClass,
    pub debit: Money,
    pub credit: Money,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    chart: BTreeMap<AccountId, ChartAccount>,
    balances: BTreeMap<AccountId, Money>,
    journal: Vec<JournalEntry>,
    next_tx: u64,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            chart: BTreeMap::new(),
            balances: BTreeMap::new(),
            journal: Vec::new(),
            next_tx: 1,
        }
    }

    pub fn add_account(&mut self, account: ChartAccount) -> SableResult<()> {
        if self.chart.contains_key(&account.id) {
            return Err(SableError::DuplicateId(account.id.to_string()));
        }
        self.balances.insert(account.id.clone(), Money::ZERO);
        self.chart.insert(account.id.clone(), account);
        Ok(())
    }

    pub fn account(&self, id: &AccountId) -> SableResult<&ChartAccount> {
        self.chart
            .get(id)
            .ok_or_else(|| SableError::AccountNotFound(id.clone()))
    }

    pub fn balance(&self, id: &AccountId) -> SableResult<Money> {
        self.account(id)?;
        Ok(*self.balances.get(id).unwrap_or(&Money::ZERO))
    }

    pub fn next_tx_id(&mut self) -> TxId {
        let id = TxId::generated(self.next_tx);
        self.next_tx += 1;
        id
    }

    pub fn post(&mut self, entry: JournalEntry) -> SableResult<()> {
        let debits = entry.total_debits();
        let credits = entry.total_credits();
        if debits != credits {
            return Err(SableError::UnbalancedJournal {
                debits: debits.cents(),
                credits: credits.cents(),
            });
        }

        for line in &entry.lines {
            self.account(&line.account)?;
        }

        for line in &entry.lines {
            let balance = self.balances.entry(line.account.clone()).or_default();
            *balance += line.signed_effect();
        }
        self.journal.push(entry);
        Ok(())
    }

    pub fn post_invoice(
        &mut self,
        day: u32,
        invoice_id: InvoiceId,
        receivable: AccountId,
        revenue: AccountId,
        amount: Money,
        memo: impl Into<String>,
    ) -> SableResult<TxId> {
        let tx_id = self.next_tx_id();
        let entry = JournalEntry::new(tx_id.clone(), day, "invoice", memo)
            .for_invoice(invoice_id)
            .with_line(JournalLine::debit(receivable, amount, "invoice receivable"))
            .with_line(JournalLine::credit(revenue, amount, "invoice revenue"));
        self.post(entry)?;
        Ok(tx_id)
    }

    pub fn post_receipt(
        &mut self,
        day: u32,
        invoice_id: InvoiceId,
        receipt_id: ReceiptId,
        cash: AccountId,
        receivable: AccountId,
        amount: Money,
    ) -> SableResult<TxId> {
        let tx_id = self.next_tx_id();
        let entry = JournalEntry::new(tx_id.clone(), day, "receipt", "payment receipt")
            .for_invoice(invoice_id)
            .for_receipt(receipt_id)
            .with_line(JournalLine::debit(cash, amount, "cash"))
            .with_line(JournalLine::credit(receivable, amount, "receivable relief"));
        self.post(entry)?;
        Ok(tx_id)
    }

    pub fn post_credit_memo(
        &mut self,
        day: u32,
        invoice_id: InvoiceId,
        receivable: AccountId,
        allowance: AccountId,
        amount: Money,
    ) -> SableResult<TxId> {
        let tx_id = self.next_tx_id();
        let entry = JournalEntry::new(tx_id.clone(), day, "adjustment", "commercial allowance")
            .for_invoice(invoice_id)
            .with_line(JournalLine::debit(allowance, amount, "allowance"))
            .with_line(JournalLine::credit(receivable, amount, "receivable relief"));
        self.post(entry)?;
        Ok(tx_id)
    }

    pub fn post_debit_memo(
        &mut self,
        day: u32,
        invoice_id: InvoiceId,
        receivable: AccountId,
        revenue: AccountId,
        amount: Money,
    ) -> SableResult<TxId> {
        let tx_id = self.next_tx_id();
        let entry = JournalEntry::new(tx_id.clone(), day, "adjustment", "commercial debit memo")
            .for_invoice(invoice_id)
            .with_line(JournalLine::debit(
                receivable,
                amount,
                "receivable increase",
            ))
            .with_line(JournalLine::credit(revenue, amount, "revenue correction"));
        self.post(entry)?;
        Ok(tx_id)
    }

    pub fn post_settlement_claim(
        &mut self,
        day: u32,
        invoice_id: InvoiceId,
        batch_id: BatchId,
        receivable: AccountId,
        clearing: AccountId,
        amount: Money,
    ) -> SableResult<TxId> {
        let tx_id = self.next_tx_id();
        let entry = JournalEntry::new(tx_id.clone(), day, "settlement", "settlement claim")
            .for_invoice(invoice_id)
            .in_batch(batch_id)
            .with_line(JournalLine::debit(clearing, amount, "settlement clearing"))
            .with_line(JournalLine::credit(
                receivable,
                amount,
                "claim presentation",
            ));
        self.post(entry)?;
        Ok(tx_id)
    }

    pub fn trial_balance(&self) -> TrialBalance {
        let mut rows = Vec::new();
        let mut total_debits = Money::ZERO;
        let mut total_credits = Money::ZERO;

        for (account_id, account) in &self.chart {
            let signed = *self.balances.get(account_id).unwrap_or(&Money::ZERO);
            let (debit, credit) = if signed.cents() >= 0 {
                (signed, Money::ZERO)
            } else {
                (Money::ZERO, Money::from_cents(-signed.cents()))
            };
            total_debits += debit;
            total_credits += credit;
            rows.push(TrialBalanceRow {
                account: account_id.clone(),
                code: account.code.clone(),
                name: account.name.clone(),
                class: account.class.clone(),
                debit,
                credit,
            });
        }

        TrialBalance {
            accounts: rows,
            total_debits,
            total_credits,
            balanced: total_debits == total_credits,
        }
    }

    pub fn journal(&self) -> &[JournalEntry] {
        &self.journal
    }

    pub fn entries_for_invoice(&self, invoice_id: &InvoiceId) -> Vec<&JournalEntry> {
        self.journal
            .iter()
            .filter(|entry| entry.invoice_id.as_ref() == Some(invoice_id))
            .collect()
    }

    pub fn account_count(&self) -> usize {
        self.chart.len()
    }
}
