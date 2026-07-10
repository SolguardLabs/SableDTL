use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessorRule {
    pub code: &'static str,
    pub lane: &'static str,
    pub description: &'static str,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TreasuryControl {
    pub code: &'static str,
    pub owner: &'static str,
    pub cadence: &'static str,
    pub evidence: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MerchantSku {
    pub sku: &'static str,
    pub family: &'static str,
    pub description: &'static str,
}

pub fn processor_rules() -> Vec<ProcessorRule> {
    vec![
        ProcessorRule {
            code: "ACH-SAME-DAY",
            lane: "bank",
            description: "Same-day ACH files received before processor cutoff",
            active: true,
        },
        ProcessorRule {
            code: "ACH-NEXT-DAY",
            lane: "bank",
            description: "Next-day ACH files received after same-day cutoff",
            active: true,
        },
        ProcessorRule {
            code: "WIRE-US",
            lane: "bank",
            description: "Domestic wire deposits matched by bank reference",
            active: true,
        },
        ProcessorRule {
            code: "CARD-BATCH",
            lane: "network",
            description: "Card network batch receipts imported from processor file",
            active: true,
        },
        ProcessorRule {
            code: "INTERNAL-OFFSET",
            lane: "internal",
            description: "Internal transfer between treasury and merchant clearing",
            active: true,
        },
        ProcessorRule {
            code: "STABLE-RAIL",
            lane: "digital",
            description: "Stablecoin rail receipt after treasury confirmation",
            active: true,
        },
    ]
}

pub fn treasury_controls() -> Vec<TreasuryControl> {
    vec![
        TreasuryControl {
            code: "TB-001",
            owner: "controller",
            cadence: "daily",
            evidence: "balanced trial balance export",
        },
        TreasuryControl {
            code: "TB-002",
            owner: "settlement-ops",
            cadence: "daily",
            evidence: "processor receipt import checksum",
        },
        TreasuryControl {
            code: "TB-003",
            owner: "treasury",
            cadence: "weekly",
            evidence: "bank account confirmation",
        },
        TreasuryControl {
            code: "TB-004",
            owner: "controller",
            cadence: "monthly",
            evidence: "close batch digest and approval",
        },
        TreasuryControl {
            code: "TB-005",
            owner: "risk",
            cadence: "monthly",
            evidence: "settlement exposure report",
        },
        TreasuryControl {
            code: "TB-006",
            owner: "audit",
            cadence: "quarterly",
            evidence: "counterparty statement sampling",
        },
    ]
}

pub fn merchant_skus() -> Vec<MerchantSku> {
    vec![
        MerchantSku {
            sku: "SBL-CLEAR-BASE",
            family: "clearing",
            description: "Base DTL clearing service bundle",
        },
        MerchantSku {
            sku: "SBL-CLEAR-PLUS",
            family: "clearing",
            description: "Enhanced DTL clearing service bundle",
        },
        MerchantSku {
            sku: "SBL-RECON-DAY",
            family: "reconciliation",
            description: "Daily processor reconciliation service",
        },
        MerchantSku {
            sku: "SBL-RECON-MONTH",
            family: "reconciliation",
            description: "Monthly accounting close support",
        },
        MerchantSku {
            sku: "SBL-SETTLE-STD",
            family: "settlement",
            description: "Standard settlement presentation service",
        },
        MerchantSku {
            sku: "SBL-SETTLE-FAST",
            family: "settlement",
            description: "Expedited settlement presentation service",
        },
        MerchantSku {
            sku: "SBL-DISPUTE-CASE",
            family: "dispute",
            description: "Case-level dispute support service",
        },
        MerchantSku {
            sku: "SBL-DISPUTE-BATCH",
            family: "dispute",
            description: "Batch dispute support service",
        },
    ]
}
