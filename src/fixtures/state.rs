use crate::adjustments::{
    AdjustmentBook, AdjustmentPolicy, AdjustmentReason, AdjustmentSide, ManualAdjustment,
};
use crate::amount::{Bps, Money};
use crate::close::{CloseBatch, Period};
use crate::error::SableResult;
use crate::ids::{AccountId, AdjustmentId, BatchId, InvoiceId, PartyId, ReceiptId};
use crate::invoice::{Invoice, InvoiceBook, LineItem, PaymentTerms};
use crate::ledger::{AccountClass, ChartAccount, Ledger};
use crate::party::{BankAccount, Counterparty, CounterpartyBook, CounterpartyRole};
use crate::policy::PolicySet;
use crate::receipts::{PaymentChannel, ReceiptBook, SettlementReceipt};
use crate::settlement::SettlementBook;

#[derive(Clone, Debug)]
pub struct AccountMap {
    pub cash: AccountId,
    pub receivable: AccountId,
    pub revenue: AccountId,
    pub allowance: AccountId,
    pub clearing: AccountId,
    pub suspense: AccountId,
}

impl AccountMap {
    pub fn standard() -> Self {
        Self {
            cash: AccountId::from("1000-cash"),
            receivable: AccountId::from("1200-accounts-receivable"),
            revenue: AccountId::from("4000-merchant-revenue"),
            allowance: AccountId::from("4090-commercial-allowance"),
            clearing: AccountId::from("1300-settlement-clearing"),
            suspense: AccountId::from("1990-reconciliation-suspense"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeedParties {
    pub merchant: PartyId,
    pub buyer_alpha: PartyId,
    pub buyer_beta: PartyId,
    pub buyer_gamma: PartyId,
    pub processor: PartyId,
    pub treasury: PartyId,
    pub auditor: PartyId,
}

impl SeedParties {
    pub fn standard() -> Self {
        Self {
            merchant: PartyId::from("merchant-sable-market"),
            buyer_alpha: PartyId::from("buyer-aurelian-retail"),
            buyer_beta: PartyId::from("buyer-bluegrain-logistics"),
            buyer_gamma: PartyId::from("buyer-cavern-supply"),
            processor: PartyId::from("processor-delta-rail"),
            treasury: PartyId::from("treasury-sable"),
            auditor: PartyId::from("auditor-northdesk"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SableState {
    pub accounts: AccountMap,
    pub parties: SeedParties,
    pub counterparties: CounterpartyBook,
    pub invoices: InvoiceBook,
    pub receipts: ReceiptBook,
    pub adjustments: AdjustmentBook,
    pub settlement: SettlementBook,
    pub ledger: Ledger,
    pub adjustment_policy: AdjustmentPolicy,
    pub policies: PolicySet,
    next_invoice: u64,
    next_receipt: u64,
    next_adjustment: u64,
    next_batch: u64,
}

pub fn seeded_state() -> SableResult<SableState> {
    let accounts = AccountMap::standard();
    let parties = SeedParties::standard();
    let mut ledger = Ledger::new();
    install_chart(&mut ledger, &accounts)?;

    let mut counterparties = CounterpartyBook::new();
    install_parties(&mut counterparties, &parties, &accounts)?;

    Ok(SableState {
        accounts,
        parties,
        counterparties,
        invoices: InvoiceBook::new(),
        receipts: ReceiptBook::new(),
        adjustments: AdjustmentBook::new(),
        settlement: SettlementBook::new(),
        ledger,
        adjustment_policy: AdjustmentPolicy::conservative(),
        policies: PolicySet::standard(),
        next_invoice: 1,
        next_receipt: 1,
        next_adjustment: 1,
        next_batch: 1,
    })
}

impl SableState {
    pub fn next_batch_id(&mut self) -> BatchId {
        let id = BatchId::generated(self.next_batch);
        self.next_batch += 1;
        id
    }

    pub fn issue_standard_invoice(
        &mut self,
        buyer: PartyId,
        external_ref: impl Into<String>,
        issue_day: u32,
        amount: Money,
        tax_bps: Bps,
    ) -> SableResult<InvoiceId> {
        let id = InvoiceId::generated(self.next_invoice);
        self.next_invoice += 1;
        let sku = format!("SBL-{}", self.next_invoice + 1000);
        let line = LineItem::new(sku, "DTL clearing service bundle", 1, amount, tax_bps);
        let mut invoice = Invoice::new(
            id.clone(),
            self.parties.merchant.clone(),
            buyer,
            external_ref,
            issue_day,
            PaymentTerms::net_30(),
            vec![line],
        )?;
        invoice.issue()?;
        self.ledger.post_invoice(
            issue_day,
            id.clone(),
            self.accounts.receivable.clone(),
            self.accounts.revenue.clone(),
            invoice.total,
            "merchant invoice",
        )?;
        self.invoices.insert(invoice)?;
        Ok(id)
    }

    pub fn record_receipt(
        &mut self,
        invoice_id: &InvoiceId,
        day: u32,
        amount: Money,
        channel: PaymentChannel,
        bank_ref: impl Into<String>,
    ) -> SableResult<ReceiptId> {
        let invoice = self.invoices.get(invoice_id)?.clone();
        let id = ReceiptId::generated(self.next_receipt);
        self.next_receipt += 1;
        let mut receipt = SettlementReceipt::new(
            id.clone(),
            invoice_id.clone(),
            invoice.buyer,
            invoice.merchant,
            channel,
            bank_ref,
            amount,
            day,
        );
        receipt.mark_matched();
        let tx_id = self.ledger.post_receipt(
            day,
            invoice_id.clone(),
            id.clone(),
            self.accounts.cash.clone(),
            self.accounts.receivable.clone(),
            amount,
        )?;
        self.invoices.apply_receipt(invoice_id, amount)?;
        receipt.mark_posted(tx_id);
        self.receipts.insert(receipt)?;
        Ok(id)
    }

    pub fn record_credit_memo(
        &mut self,
        invoice_id: &InvoiceId,
        day: u32,
        amount: Money,
        reason: AdjustmentReason,
    ) -> SableResult<AdjustmentId> {
        self.record_adjustment(invoice_id, day, amount, reason, AdjustmentSide::CreditMemo)
    }

    pub fn record_debit_memo(
        &mut self,
        invoice_id: &InvoiceId,
        day: u32,
        amount: Money,
        reason: AdjustmentReason,
    ) -> SableResult<AdjustmentId> {
        self.record_adjustment(invoice_id, day, amount, reason, AdjustmentSide::DebitMemo)
    }

    pub fn record_adjustment(
        &mut self,
        invoice_id: &InvoiceId,
        day: u32,
        amount: Money,
        reason: AdjustmentReason,
        side: AdjustmentSide,
    ) -> SableResult<AdjustmentId> {
        let id = AdjustmentId::generated(self.next_adjustment);
        self.next_adjustment += 1;
        let base = match side {
            AdjustmentSide::CreditMemo => ManualAdjustment::credit(
                id.clone(),
                invoice_id.clone(),
                reason,
                amount,
                self.parties.processor.clone(),
                day,
            ),
            AdjustmentSide::DebitMemo => ManualAdjustment::debit(
                id.clone(),
                invoice_id.clone(),
                reason,
                amount,
                self.parties.processor.clone(),
                day,
            ),
        };
        let adjustment = base
            .approve(self.parties.treasury.clone())
            .with_memo("periodic reconciliation memo");
        self.adjustments.apply(
            adjustment,
            &self.adjustment_policy,
            &mut self.invoices,
            &mut self.ledger,
            self.accounts.receivable.clone(),
            self.accounts.revenue.clone(),
            self.accounts.allowance.clone(),
        )?;
        Ok(id)
    }

    pub fn refresh_settlement(&mut self) {
        self.settlement.refresh(&self.invoices);
    }

    pub fn prepare_and_post_settlement(&mut self, day: u32) -> SableResult<Vec<String>> {
        self.refresh_settlement();
        let batch_id = self.next_batch_id();
        let claim_ids = self.settlement.prepare_due_claims(day, batch_id.clone())?;
        self.settlement.post_claims(
            &claim_ids,
            day,
            &mut self.ledger,
            self.accounts.receivable.clone(),
            self.accounts.clearing.clone(),
        )?;
        Ok(claim_ids.into_iter().map(|id| id.to_string()).collect())
    }

    pub fn prepare_close(&mut self, period: Period, prepared_day: u32) -> SableResult<CloseBatch> {
        self.refresh_settlement();
        let batch_id = self.next_batch_id();
        CloseBatch::prepare(
            batch_id,
            period,
            prepared_day,
            &self.invoices,
            &self.receipts,
            &self.settlement,
            &self.ledger,
        )
    }

    pub fn close_eligible_invoices(&mut self, day: u32) -> SableResult<usize> {
        let ids: Vec<_> = self
            .invoices
            .eligible_for_close(day)
            .into_iter()
            .map(|invoice| invoice.id.clone())
            .collect();
        for id in &ids {
            self.invoices.close_invoice(id, day)?;
        }
        Ok(ids.len())
    }
}

fn install_chart(ledger: &mut Ledger, accounts: &AccountMap) -> SableResult<()> {
    ledger.add_account(ChartAccount::new(
        accounts.cash.clone(),
        "1000",
        "Operating cash",
        AccountClass::Asset,
    ))?;
    ledger.add_account(ChartAccount::new(
        accounts.receivable.clone(),
        "1200",
        "Accounts receivable",
        AccountClass::Asset,
    ))?;
    ledger.add_account(ChartAccount::new(
        accounts.clearing.clone(),
        "1300",
        "Settlement clearing",
        AccountClass::Asset,
    ))?;
    ledger.add_account(ChartAccount::new(
        accounts.suspense.clone(),
        "1990",
        "Reconciliation suspense",
        AccountClass::Clearing,
    ))?;
    ledger.add_account(ChartAccount::new(
        accounts.revenue.clone(),
        "4000",
        "Merchant revenue",
        AccountClass::Revenue,
    ))?;
    ledger.add_account(ChartAccount::new(
        accounts.allowance.clone(),
        "4090",
        "Commercial allowances",
        AccountClass::ContraRevenue,
    ))?;
    Ok(())
}

fn install_parties(
    book: &mut CounterpartyBook,
    parties: &SeedParties,
    accounts: &AccountMap,
) -> SableResult<()> {
    book.insert(
        Counterparty::new(
            parties.merchant.clone(),
            "Sable Market Labs Ltd.",
            CounterpartyRole::Merchant,
            accounts.cash.clone(),
            accounts.receivable.clone(),
            accounts.suspense.clone(),
        )
        .with_bank_account(BankAccount::new(
            "Northline Bank",
            "NLINUS33",
            "00044122",
            "USD",
        )),
    )?;
    book.insert(
        Counterparty::new(
            parties.buyer_alpha.clone(),
            "Aurelian Retail Group",
            CounterpartyRole::Buyer,
            accounts.cash.clone(),
            accounts.suspense.clone(),
            accounts.receivable.clone(),
        )
        .with_bank_account(BankAccount::new(
            "Pioneer Bank",
            "PIONUS44",
            "99120001",
            "USD",
        )),
    )?;
    book.insert(
        Counterparty::new(
            parties.buyer_beta.clone(),
            "Bluegrain Logistics LLC",
            CounterpartyRole::Buyer,
            accounts.cash.clone(),
            accounts.suspense.clone(),
            accounts.receivable.clone(),
        )
        .with_bank_account(BankAccount::new(
            "Harbor Trust",
            "HARBUS55",
            "10770021",
            "USD",
        )),
    )?;
    book.insert(
        Counterparty::new(
            parties.buyer_gamma.clone(),
            "Cavern Supply Cooperative",
            CounterpartyRole::Buyer,
            accounts.cash.clone(),
            accounts.suspense.clone(),
            accounts.receivable.clone(),
        )
        .with_bank_account(BankAccount::new(
            "Redstone Credit",
            "REDCUS66",
            "30241077",
            "USD",
        )),
    )?;
    book.insert(Counterparty::new(
        parties.processor.clone(),
        "Delta Rail Processor",
        CounterpartyRole::Processor,
        accounts.clearing.clone(),
        accounts.suspense.clone(),
        accounts.receivable.clone(),
    ))?;
    book.insert(Counterparty::new(
        parties.treasury.clone(),
        "Sable Treasury Desk",
        CounterpartyRole::Treasury,
        accounts.cash.clone(),
        accounts.suspense.clone(),
        accounts.receivable.clone(),
    ))?;
    book.insert(Counterparty::new(
        parties.auditor.clone(),
        "Northdesk Audit Office",
        CounterpartyRole::Auditor,
        accounts.suspense.clone(),
        accounts.suspense.clone(),
        accounts.suspense.clone(),
    ))?;
    Ok(())
}
