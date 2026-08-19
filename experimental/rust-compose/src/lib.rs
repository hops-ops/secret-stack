//! Experimental SecretStack composition with layered existence gates.
//!
//! This crate re-expresses `functions/render/*.gotmpl` in Rust:
//!
//! | gotmpl | Rust |
//! |--------|------|
//! | `000-state-init` | [`state::EffectiveState::from_spec`] |
//! | `010-state-status` | [`state::Observed`] + [`state::compute_status`] |
//! | `200` / `201` / `210` / `230` | [`compose::compose`] + [`resources`] |
//! | `$shouldRender` / `if exists` | [`Desired::under_exists`](desired::Desired::under_exists) |
//! | Usage `if ready` | [`Desired::usage_when_ready`](desired::Desired::usage_when_ready) |
//!
//! **Not production.** Not wired to `composition.yaml`. No gRPC function package yet.
//! Goal: prove the gate DX on a real stack before investing in a function runtime.

pub mod compose;
pub mod desired;
pub mod gate;
pub mod resources;
pub mod state;

pub use compose::{compose, ComposeResult};
pub use desired::Desired;
pub use gate::{Exists, ObservedSlice, Ready};
pub use state::{Backend, EffectiveState, Observed, SecretStackSpec, SecretStoreScope};
