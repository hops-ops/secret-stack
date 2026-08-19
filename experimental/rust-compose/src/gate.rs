//! Sticky existence vs readiness.
//!
//! Matches xrd-authoring observed-state pattern:
//! - **Exists** — sticky atProvider signal (Helm `revision > 0`, upjet id/arn, …).
//!   Use only for **dependent resource render** gates. Un-render deletes MRs.
//! - **Ready** — condition Ready=True. Use for **status** and **Usage** only.

/// Sticky existence. Safe default for dependent MR render gates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Exists(pub bool);

impl Exists {
    pub const YES: Self = Self(true);
    pub const NO: Self = Self(false);

    #[inline]
    pub fn is_set(self) -> bool {
        self.0
    }

    /// Helm sticky signal: revision > 0 means installed at least once.
    pub fn from_helm_revision(revision: i64) -> Self {
        Self(revision > 0)
    }

    /// Non-empty sticky id/arn (upjet).
    pub fn from_nonempty(s: Option<&str>) -> Self {
        Self(s.map(|v| !v.is_empty()).unwrap_or(false))
    }

    /// Resource key present in observed map (weaker; prefer sticky fields).
    pub fn from_observed_entry(present: bool) -> Self {
        Self(present)
    }
}

impl std::ops::BitAnd for Exists {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 && rhs.0)
    }
}

/// Condition Ready=True. Not for dependent MR render gates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Ready(pub bool);

impl Ready {
    pub const YES: Self = Self(true);
    pub const NO: Self = Self(false);

    #[inline]
    pub fn is_set(self) -> bool {
        self.0
    }
}

impl std::ops::BitAnd for Ready {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 && rhs.0)
    }
}

/// Observed slice for one composed resource: both signals, always.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ObservedSlice {
    pub exists: Exists,
    pub ready: Ready,
}

impl ObservedSlice {
    pub fn new(exists: Exists, ready: Ready) -> Self {
        Self { exists, ready }
    }

    pub fn missing() -> Self {
        Self {
            exists: Exists::NO,
            ready: Ready::NO,
        }
    }

    pub fn helm(revision: i64, ready: bool) -> Self {
        Self {
            exists: Exists::from_helm_revision(revision),
            ready: Ready(ready),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helm_revision_zero_is_not_exists() {
        assert!(!Exists::from_helm_revision(0).is_set());
        assert!(Exists::from_helm_revision(1).is_set());
    }

    #[test]
    fn exists_and_ready_are_distinct() {
        let mid_upgrade = ObservedSlice::helm(3, false);
        assert!(mid_upgrade.exists.is_set());
        assert!(!mid_upgrade.ready.is_set());
    }
}
