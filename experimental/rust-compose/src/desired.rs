//! Desired composed resources + layered existence gates.
//!
//! ```ignore
//! d.emit("helm-external-secrets", eso);
//! d.under_exists(obs.helm_external_secrets.exists, |d| {
//!     d.emit("secret-store", store);
//! });
//! d.usage_when_ready(obs.a.ready & obs.b.ready, "usage-name", usage);
//! ```

use indexmap::IndexMap;
use serde_json::Value;

use crate::gate::{Exists, Ready};

/// Stable composition resource name (`crossplane.io/composition-resource-name`).
pub type ResourceName = String;

/// One desired composed resource (unstructured JSON body for the prototype).
#[derive(Debug, Clone, PartialEq)]
pub struct DesiredResource {
    pub body: Value,
}

/// Accumulated desired set for one RunFunction response (prototype).
#[derive(Debug, Default, Clone)]
pub struct Desired {
    resources: IndexMap<ResourceName, DesiredResource>,
}

impl Desired {
    pub fn new() -> Self {
        Self::default()
    }

    /// Always emit into desired (root layer — no parent gate).
    pub fn emit(&mut self, name: impl Into<ResourceName>, body: Value) {
        self.resources.insert(name.into(), DesiredResource { body });
    }

    /// Layer: run `f` only when sticky existence is set.
    ///
    /// If `gate` is false, children are **not** added — matching un-render
    /// semantics. Callers must use sticky Exists, never Ready.
    pub fn under_exists(&mut self, gate: Exists, f: impl FnOnce(&mut Desired)) {
        if gate.is_set() {
            f(self);
        }
    }

    /// Usage / ordering lock: gate on Ready (deliberate exception).
    pub fn usage_when_ready(&mut self, gate: Ready, name: impl Into<ResourceName>, body: Value) {
        if gate.is_set() {
            self.emit(name, body);
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.resources.contains_key(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.resources.keys().map(|s| s.as_str()).collect()
    }

    pub fn get(&self, name: &str) -> Option<&DesiredResource> {
        self.resources.get(name)
    }

    pub fn len(&self) -> usize {
        self.resources.len()
    }

    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn under_exists_skips_children_when_missing() {
        let mut d = Desired::new();
        d.emit("vpc", json!({"kind": "VPC"}));
        d.under_exists(Exists::NO, |d| {
            d.emit("subnet", json!({"kind": "Subnet"}));
        });
        assert!(d.contains("vpc"));
        assert!(!d.contains("subnet"));
    }

    #[test]
    fn under_exists_emits_children_when_present() {
        let mut d = Desired::new();
        d.under_exists(Exists::YES, |d| {
            d.emit("subnet", json!({"kind": "Subnet"}));
        });
        assert!(d.contains("subnet"));
    }

    #[test]
    fn usage_when_ready_requires_ready() {
        let mut d = Desired::new();
        d.usage_when_ready(Ready::NO, "usage-x", json!({"kind": "Usage"}));
        assert!(!d.contains("usage-x"));
        d.usage_when_ready(Ready::YES, "usage-x", json!({"kind": "Usage"}));
        assert!(d.contains("usage-x"));
    }
}
