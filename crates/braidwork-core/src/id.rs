//! Immutable, consumer-supplied identifiers with validated string representations.
//!
//! Identifiers must be non-empty and have no leading or trailing whitespace.
//! Whitespace-only strings are rejected; internal whitespace is permitted.
//! Valid strings are preserved: validation does not trim, normalize, or generate IDs.
//! Deserialization applies the same validation as ordinary construction.

use std::{error::Error, fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// An identifier was empty, whitespace-only, or had leading or trailing whitespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidId;

impl fmt::Display for InvalidId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("an identifier must be non-empty and have no leading or trailing whitespace")
    }
}

impl Error for InvalidId {}

// These newtypes have exactly the same validation and string operations.
// A single local macro keeps deserialization and constructors consistent.
macro_rules! define_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            /// Validates a consumer-supplied identifier without normalizing it.
            ///
            /// # Errors
            /// Returns [`InvalidId`] for an empty or whitespace-only string,
            /// or one with leading or trailing whitespace. Internal whitespace
            /// is permitted and valid strings are preserved exactly.
            pub fn new(value: impl Into<String>) -> Result<Self, InvalidId> {
                let value = value.into();
                if value.is_empty() || value.trim() != value.as_str() {
                    return Err(InvalidId);
                }
                Ok(Self(value))
            }

            /// Borrows the validated identifier as a string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidId;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = InvalidId;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl FromStr for $name {
            type Err = InvalidId;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::new(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

define_id!(
    ResourceId,
    "Identifies an AI resource, independently of its provider or model."
);
define_id!(
    ModelId,
    "Identifies a concrete execution model, separately from the resource providing access."
);
define_id!(TaskId, "Identifies a unit of project work.");
define_id!(
    CapsuleId,
    "Identifies a particular portable task description."
);
define_id!(
    ArtifactId,
    "Identifies an artifact's metadata, separately from its content reference."
);
define_id!(
    ReceiptId,
    "Identifies an auditable execution and verification record."
);
