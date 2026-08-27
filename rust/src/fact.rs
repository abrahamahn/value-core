use std::collections::{BTreeMap, BTreeSet};

use crate::time::parse_rfc3339_millis;
use crate::{ValueError, ValueResult, is_lower_sha256, required};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedValueFact {
    pub id: String,
    pub sequence: u64,
    pub occurred_at: String,
}

/// Sort value facts deterministically and enforce a positive, gap-free sequence.
///
/// # Errors
/// Returns [`ValueError`] when an identity, sequence, or timestamp is invalid.
pub fn order_value_facts(facts: &[OrderedValueFact]) -> ValueResult<Vec<OrderedValueFact>> {
    for fact in facts {
        required(&fact.id, "Value fact identity is required")?;
        if fact.sequence == 0 {
            return Err(ValueError::new(
                "Value fact sequence must be a positive integer",
            ));
        }
        parse_rfc3339_millis(&fact.occurred_at, "Value fact timestamp")?;
    }
    let mut ordered = facts.to_vec();
    ordered.sort_by(|left, right| {
        left.sequence
            .cmp(&right.sequence)
            .then(left.id.cmp(&right.id))
    });
    for (index, fact) in ordered.iter().enumerate() {
        if fact.sequence != u64::try_from(index + 1).unwrap_or(u64::MAX) {
            return Err(ValueError::new(
                "Value fact sequence must be positive and gap-free",
            ));
        }
    }
    Ok(ordered)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PinnedDigestReferences {
    pub digest: String,
    pub consistent: bool,
}

/// Validate that every caller-selected artifact pins one canonical digest.
///
/// # Errors
/// Returns [`ValueError`] when the digest, required names, or references are invalid.
pub fn validate_pinned_digest_references(
    digest: &str,
    required_references: &[String],
    references: &BTreeMap<String, String>,
) -> ValueResult<PinnedDigestReferences> {
    if !is_lower_sha256(digest) {
        return Err(ValueError::new(
            "Pinned value digest must be lowercase SHA-256",
        ));
    }
    let mut required_names = BTreeSet::new();
    for name in required_references {
        required(
            name,
            "Pinned value reference names must be non-empty and unique",
        )?;
        if !required_names.insert(name) {
            return Err(ValueError::new(
                "Pinned value reference names must be non-empty and unique",
            ));
        }
        match references.get(name) {
            None => {
                return Err(ValueError::new(format!(
                    "Pinned value reference {name} is missing"
                )));
            }
            Some(reference) if reference != digest => {
                return Err(ValueError::new(format!(
                    "Pinned value reference {name} digest mismatch"
                )));
            }
            Some(_) => {}
        }
    }
    Ok(PinnedDigestReferences {
        digest: digest.to_owned(),
        consistent: true,
    })
}
