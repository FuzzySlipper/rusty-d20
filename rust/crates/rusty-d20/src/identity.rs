use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

pub const MAX_D20_ID_BYTES: usize = 64;
pub const D20_ID_PATTERN: &str = "^[a-z0-9._-]+$";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, TS)]
pub struct D20Id(String);

impl D20Id {
    pub fn parse(value: impl Into<String>) -> Result<Self, D20IdentityError> {
        let value = value.into();
        if value.is_empty() {
            return Err(D20IdentityError::Empty);
        }
        if value.len() > MAX_D20_ID_BYTES {
            return Err(D20IdentityError::TooLong {
                actual: value.len(),
                maximum: MAX_D20_ID_BYTES,
            });
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        }) {
            return Err(D20IdentityError::InvalidCharacter { value });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for D20Id {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for D20Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for D20Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D20IdentityError {
    Empty,
    TooLong { actual: usize, maximum: usize },
    InvalidCharacter { value: String },
}

impl fmt::Display for D20IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid d20 identity: {self:?}")
    }
}

impl std::error::Error for D20IdentityError {}
