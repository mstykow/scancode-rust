// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct DependencyUid(String);

impl DependencyUid {
    /// Creates a new `DependencyUid` by appending a UUID to the given purl.
    pub fn new(purl: &str) -> Self {
        let uuid = Uuid::new_v4();
        DependencyUid(crate::models::purl::append_uuid_qualifier(
            purl,
            &uuid.to_string(),
        ))
    }

    /// Wraps an existing UID string without validation or UUID generation.
    ///
    /// Use this for deserialization boundaries and round-trip conversions
    /// where the UID string is already well-formed.
    pub fn from_raw(s: String) -> Self {
        DependencyUid(s)
    }

    /// Returns the empty-string sentinel representing "no purl".
    pub fn empty() -> Self {
        DependencyUid(String::new())
    }

    /// Returns a new `DependencyUid` with the purl base replaced, preserving the UUID.
    pub fn replace_base(&self, new_purl: &str) -> Self {
        let Some(uuid) = crate::models::purl::uuid_qualifier_value(&self.0) else {
            return DependencyUid(self.0.clone());
        };
        DependencyUid(crate::models::purl::append_uuid_qualifier(new_purl, uuid))
    }
}

impl fmt::Display for DependencyUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for DependencyUid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for DependencyUid {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Deref for DependencyUid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
