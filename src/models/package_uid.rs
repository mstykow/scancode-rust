// SPDX-FileCopyrightText: Provenant contributors
// SPDX-License-Identifier: Apache-2.0

use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PackageUid(String);

impl PackageUid {
    /// Creates a new `PackageUid` by appending a UUID to the given purl.
    pub fn new(purl: &str) -> Self {
        let uuid = Uuid::new_v4();
        Self::with_uuid_suffix(purl, uuid)
    }

    /// Creates a new `PackageUid` from a non-purl base string.
    pub fn new_opaque(base: &str) -> Self {
        let uuid = Uuid::new_v4();
        Self::with_uuid_suffix(base, uuid)
    }

    fn with_uuid_suffix(base: &str, uuid: Uuid) -> Self {
        PackageUid(crate::models::purl::append_uuid_qualifier(
            base,
            &uuid.to_string(),
        ))
    }

    /// Wraps an existing UID string without validation or UUID generation.
    ///
    /// Use this for deserialization boundaries and round-trip conversions
    /// where the UID string is already well-formed.
    pub fn from_raw(s: String) -> Self {
        PackageUid(s)
    }

    /// Returns the empty-string sentinel representing "no purl".
    pub fn empty() -> Self {
        PackageUid(String::new())
    }

    /// Returns the purl portion by stripping the UUID qualifier. Borrows unless
    /// the purl carries a subpath, which has to be rejoined to what precedes the
    /// qualifier.
    pub fn stable_key(&self) -> std::borrow::Cow<'_, str> {
        crate::models::purl::strip_uuid_qualifier(&self.0)
    }

    /// Returns a new `PackageUid` with the purl base replaced, preserving the UUID.
    pub fn replace_base(&self, new_purl: &str) -> Self {
        let Some(uuid) = crate::models::purl::uuid_qualifier_value(&self.0) else {
            return PackageUid(self.0.clone());
        };
        PackageUid(crate::models::purl::append_uuid_qualifier(new_purl, uuid))
    }

    /// Returns the inner string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns `true` if this is the empty-string sentinel.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for PackageUid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for PackageUid {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for PackageUid {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl Deref for PackageUid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
