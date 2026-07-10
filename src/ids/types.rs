use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn generated(sequence: u64) -> Self {
                Self(format!("{}-{sequence:08}", $prefix))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(AccountId, "acct");
id_type!(AdjustmentId, "adj");
id_type!(BatchId, "batch");
id_type!(InvoiceId, "inv");
id_type!(PartyId, "party");
id_type!(ReceiptId, "rcpt");
id_type!(SettlementId, "set");
id_type!(TxId, "tx");
