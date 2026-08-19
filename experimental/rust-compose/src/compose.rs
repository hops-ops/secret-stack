//! SecretStack layered composition.
//!
//! Gate graph (same as `functions/render/*.gotmpl`):
//!
//! ```text
//! [root — always]
//!   helm-external-secrets
//!   helm-vault                 if vault.install
//!   pod-identity               if backend=aws
//!   usage-pod-identity         if backend=aws AND both Ready
//!
//! [under ESO exists (revision > 0)]
//!   + vault exists if vault.install
//!   + aws.region non-empty if backend=aws
//!   + vault.server non-empty if backend=vault
//!     secret-store
//!     usage-secret-store-helm  if secret-store Ready
//!     usage-secret-store-vault if vault.install AND both Ready
//! ```
//!
//! Existence gates dependents; Ready gates Usages only.

use crate::desired::Desired;
use crate::gate::Exists;
use crate::resources;
use crate::state::{all_ready, compute_status, EffectiveState, Observed, StatusOut};

/// Resource name constants — must stay stable across reconciles.
pub mod names {
    pub const HELM_ESO: &str = "helm-external-secrets";
    pub const HELM_VAULT: &str = "helm-vault";
    pub const POD_IDENTITY: &str = "pod-identity";
    pub const USAGE_POD_IDENTITY: &str = "usage-pod-identity";
    pub const SECRET_STORE: &str = "secret-store";
    pub const USAGE_SECRET_STORE_HELM: &str = "usage-secret-store-helm";
    pub const USAGE_SECRET_STORE_VAULT: &str = "usage-secret-store-vault";
}

pub struct ComposeResult {
    pub desired: Desired,
    pub status: StatusOut,
}

/// Compose desired resources for one SecretStack reconcile.
pub fn compose(state: &EffectiveState, obs: &Observed) -> ComposeResult {
    let mut d = Desired::new();

    // --- root layer (no parent existence gate) ---
    d.emit(names::HELM_ESO, resources::helm_external_secrets(state));

    if state.vault_install {
        d.emit(names::HELM_VAULT, resources::helm_vault(state));
    }

    if state.aws_enabled {
        d.emit(names::POD_IDENTITY, resources::pod_identity(state));
        // Usage: Ready-gated (delete ESO Helm before PodIdentity)
        d.usage_when_ready(
            obs.helm_external_secrets.ready & obs.pod_identity.ready,
            names::USAGE_POD_IDENTITY,
            resources::usage_pod_identity_protects_until_helm_gone(state),
        );
    }

    // --- secret-store layer: sticky existence of ESO (+ vault if installed) ---
    // Mirrors 230-secret-store.yaml.gotmpl $shouldRender
    let eso_exists = obs.helm_external_secrets.exists;
    let vault_exists_if_needed = if state.vault_install {
        obs.helm_vault.exists
    } else {
        Exists::YES
    };

    let mut store_gate =
        state.secret_store_enabled && eso_exists.is_set() && vault_exists_if_needed.is_set();

    if store_gate && state.vault_enabled {
        store_gate = store_gate && !state.vault_server.is_empty();
    }
    if store_gate && state.aws_enabled {
        store_gate = store_gate && !state.aws_region.is_empty();
    }

    d.under_exists(Exists(store_gate), |d| {
        d.emit(names::SECRET_STORE, resources::secret_store_object(state));

        // Usages: Ready only
        d.usage_when_ready(
            obs.secret_store.ready,
            names::USAGE_SECRET_STORE_HELM,
            resources::usage_secret_store_before_eso_helm(state),
        );

        if state.vault_install {
            d.usage_when_ready(
                all_ready(&[obs.secret_store.ready, obs.helm_vault.ready]),
                names::USAGE_SECRET_STORE_VAULT,
                resources::usage_secret_store_before_vault(state),
            );
        }
    });

    let status = compute_status(state, obs);
    ComposeResult { desired: d, status }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::{Exists, ObservedSlice, Ready};
    use crate::state::{Backend, SecretStackSpec, SecretStoreScope};
    use pretty_assertions::assert_eq;

    fn aws_spec() -> SecretStackSpec {
        SecretStackSpec {
            metadata_name: "external-secrets".into(),
            cluster_name: "production-cluster".into(),
            backend: Backend::Aws,
            aws_region: Some("us-west-2".into()),
            aws_role_prefix: Some("prod-".into()),
            ..Default::default()
        }
    }

    fn vault_install_spec() -> SecretStackSpec {
        SecretStackSpec {
            metadata_name: "external-secrets".into(),
            cluster_name: "dory".into(),
            backend: Backend::Vault,
            vault_install: true,
            secret_store_scope: Some(SecretStoreScope::Cluster),
            secret_store_name: Some("vault".into()),
            ..Default::default()
        }
    }

    #[test]
    fn aws_bootstrap_no_secret_store_yet() {
        let state = EffectiveState::from_spec(&aws_spec());
        let obs = Observed::bootstrap();
        let r = compose(&state, &obs);

        assert!(r.desired.contains(names::HELM_ESO));
        assert!(r.desired.contains(names::POD_IDENTITY));
        assert!(!r.desired.contains(names::HELM_VAULT));
        assert!(
            !r.desired.contains(names::SECRET_STORE),
            "SecretStore must wait for ESO Helm exists (revision>0)"
        );
        assert!(!r.desired.contains(names::USAGE_POD_IDENTITY));
        assert!(!r.desired.contains(names::USAGE_SECRET_STORE_HELM));
    }

    #[test]
    fn aws_eso_exists_unlocks_secret_store_even_if_not_ready() {
        // Chart upgrade blip: revision sticky, Ready false — store must stay
        let state = EffectiveState::from_spec(&aws_spec());
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(2, false),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);

        assert!(r.desired.contains(names::SECRET_STORE));
        assert!(
            !r.desired.contains(names::USAGE_SECRET_STORE_HELM),
            "Usage still waits for secret-store Ready"
        );
    }

    #[test]
    fn aws_full_ready_emits_usages() {
        let state = EffectiveState::from_spec(&aws_spec());
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            pod_identity: ObservedSlice::new(Exists::YES, Ready::YES),
            secret_store: ObservedSlice::new(Exists::YES, Ready::YES),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);

        let mut names = r.desired.names();
        names.sort();
        assert_eq!(
            names,
            vec![
                names::HELM_ESO,
                names::POD_IDENTITY,
                names::SECRET_STORE,
                names::USAGE_POD_IDENTITY,
                names::USAGE_SECRET_STORE_HELM,
            ]
        );
    }

    #[test]
    fn vault_install_bootstrap_no_store_until_both_helms_exist() {
        let state = EffectiveState::from_spec(&vault_install_spec());
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            // vault not installed yet
            helm_vault: ObservedSlice::missing(),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);

        assert!(r.desired.contains(names::HELM_ESO));
        assert!(r.desired.contains(names::HELM_VAULT));
        assert!(!r.desired.contains(names::POD_IDENTITY));
        assert!(
            !r.desired.contains(names::SECRET_STORE),
            "vault.install requires helm-vault exists before SecretStore"
        );
    }

    #[test]
    fn vault_install_both_exist_unlocks_store() {
        let state = EffectiveState::from_spec(&vault_install_spec());
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            helm_vault: ObservedSlice::helm(1, false), // not ready — still ok for render
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);

        assert!(r.desired.contains(names::SECRET_STORE));
        assert!(!r.desired.contains(names::USAGE_SECRET_STORE_VAULT));
    }

    #[test]
    fn vault_install_usages_when_ready() {
        let state = EffectiveState::from_spec(&vault_install_spec());
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            helm_vault: ObservedSlice::helm(1, true),
            secret_store: ObservedSlice::new(Exists::YES, Ready::YES),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);

        assert!(r.desired.contains(names::USAGE_SECRET_STORE_HELM));
        assert!(r.desired.contains(names::USAGE_SECRET_STORE_VAULT));
    }

    #[test]
    fn aws_missing_region_blocks_secret_store() {
        let mut spec = aws_spec();
        spec.aws_region = Some("".into());
        let state = EffectiveState::from_spec(&spec);
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);
        assert!(!r.desired.contains(names::SECRET_STORE));
    }

    #[test]
    fn secret_store_disabled() {
        let mut spec = aws_spec();
        spec.secret_store_enabled = Some(false);
        let state = EffectiveState::from_spec(&spec);
        let obs = Observed {
            helm_external_secrets: ObservedSlice::helm(1, true),
            ..Observed::bootstrap()
        };
        let r = compose(&state, &obs);
        assert!(!r.desired.contains(names::SECRET_STORE));
    }
}
