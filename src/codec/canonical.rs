use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use crate::error::{SableError, SableResult};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Digest(String);

impl Digest {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn canonical_json<T: Serialize>(value: &T) -> SableResult<String> {
    serde_json::to_string(value).map_err(|error| SableError::Serialization(error.to_string()))
}

pub fn canonical_digest<T: Serialize>(value: &T) -> SableResult<Digest> {
    let json = canonical_json(value)?;
    Ok(Digest::new(
        blake3::hash(json.as_bytes()).to_hex().to_string(),
    ))
}
