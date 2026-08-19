//! Effective SecretStack state — mirrors `000-state-init.yaml.gotmpl`.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::gate::{ObservedSlice, Ready};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    #[default]
    Aws,
    Vault,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SecretStoreScope {
    #[default]
    Namespaced,
    Cluster,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfigRef {
    pub name: String,
    pub kind: String,
}

impl ProviderConfigRef {
    pub fn new(name: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveState {
    pub name: String,
    pub cluster_name: String,
    pub namespace: String,
    pub management_policies: Vec<String>,
    pub labels: Map<String, Value>,
    pub backend: Backend,

    pub helm_release_name: String,
    pub helm_namespace: String,
    pub helm_provider: ProviderConfigRef,
    pub helm_values: Value,
    pub helm_override_all: Option<Value>,

    pub k8s_provider: ProviderConfigRef,

    pub aws_enabled: bool,
    pub aws_region: String,
    pub aws_provider: ProviderConfigRef,
    pub aws_permissions_boundary_arn: String,
    pub aws_role_prefix: String,
    pub aws_tags: Map<String, Value>,

    pub vault_enabled: bool,
    pub vault_install: bool,
    pub vault_namespace: String,
    pub vault_release_name: String,
    pub vault_server: String,
    pub vault_path: String,
    pub vault_version: String,
    pub vault_auth_method: String,
    pub vault_auth_mount_path: String,
    pub vault_auth_role: String,
    pub vault_token_secret_name: String,
    pub vault_token_secret_key: String,
    pub vault_token_secret_namespace: String,
    pub vault_values: Value,
    pub vault_override_all: Option<Value>,

    pub secret_store_enabled: bool,
    pub secret_store_scope: SecretStoreScope,
    pub secret_store_name: String,

    pub pod_identity_name: String,
    pub service_account_name: String,
    pub service_account_namespace: String,
}

/// Minimal XR input for effective-state construction (subset of XRD).
#[derive(Debug, Clone, Default)]
pub struct SecretStackSpec {
    pub metadata_name: String,
    pub cluster_name: String,
    pub backend: Backend,
    pub namespace: Option<String>,
    pub release_name: Option<String>,
    pub labels: Map<String, Value>,
    pub management_policies: Option<Vec<String>>,
    pub values: Value,
    pub override_all_values: Option<Value>,

    pub helm_provider_name: Option<String>,
    pub k8s_provider_name: Option<String>,
    pub aws_provider_name: Option<String>,

    pub aws_region: Option<String>,
    pub aws_permissions_boundary_arn: Option<String>,
    pub aws_role_prefix: Option<String>,
    pub aws_tags: Map<String, Value>,

    pub vault_install: bool,
    pub vault_namespace: Option<String>,
    pub vault_release_name: Option<String>,
    pub vault_server: Option<String>,
    pub vault_path: Option<String>,
    pub vault_version: Option<String>,
    pub vault_auth_method: Option<String>,
    pub vault_auth_mount_path: Option<String>,
    pub vault_auth_role: Option<String>,
    pub vault_token_secret_name: Option<String>,
    pub vault_token_secret_key: Option<String>,
    pub vault_token_secret_namespace: Option<String>,
    pub vault_values: Value,
    pub vault_override_all: Option<Value>,

    pub secret_store_enabled: Option<bool>,
    pub secret_store_scope: Option<SecretStoreScope>,
    pub secret_store_name: Option<String>,
}

impl EffectiveState {
    pub fn from_spec(spec: &SecretStackSpec) -> Self {
        let name = if spec.metadata_name.is_empty() {
            "external-secrets".into()
        } else {
            spec.metadata_name.clone()
        };
        let cluster_name = if spec.cluster_name.is_empty() {
            name.clone()
        } else {
            spec.cluster_name.clone()
        };
        let namespace = spec
            .namespace
            .clone()
            .unwrap_or_else(|| "external-secrets".into());
        let backend = spec.backend;

        let mut labels = Map::new();
        labels.insert("hops.ops.com.ai/managed".into(), json!("true"));
        labels.insert(format!("hops.ops.com.ai/{}", "secretstack"), json!(name));
        for (k, v) in &spec.labels {
            labels.insert(k.clone(), v.clone());
        }

        let mut aws_tags = labels.clone();
        for (k, v) in &spec.aws_tags {
            aws_tags.insert(k.clone(), v.clone());
        }

        let vault_enabled = matches!(backend, Backend::Vault);
        let vault_install = vault_enabled && spec.vault_install;
        let vault_namespace = spec
            .vault_namespace
            .clone()
            .unwrap_or_else(|| "vault".into());
        let vault_release_name = spec
            .vault_release_name
            .clone()
            .unwrap_or_else(|| "vault".into());
        let default_vault_server = format!(
            "http://{}.{}.svc.cluster.local:8200",
            vault_release_name, vault_namespace
        );
        let vault_server = spec
            .vault_server
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| vault_install.then_some(default_vault_server))
            .unwrap_or_default();

        let vault_auth_method = spec.vault_auth_method.clone().unwrap_or_else(|| {
            if vault_install {
                "kubernetes".into()
            } else {
                "token".into()
            }
        });

        Self {
            name: name.clone(),
            cluster_name: cluster_name.clone(),
            namespace: namespace.clone(),
            management_policies: spec
                .management_policies
                .clone()
                .unwrap_or_else(|| vec!["*".into()]),
            labels,
            backend,

            helm_release_name: spec
                .release_name
                .clone()
                .unwrap_or_else(|| "external-secrets".into()),
            helm_namespace: namespace.clone(),
            helm_provider: ProviderConfigRef::new(
                spec.helm_provider_name
                    .clone()
                    .unwrap_or_else(|| cluster_name.clone()),
                "ProviderConfig",
            ),
            helm_values: if spec.values.is_null() {
                json!({})
            } else {
                spec.values.clone()
            },
            helm_override_all: spec.override_all_values.clone(),

            k8s_provider: ProviderConfigRef::new(
                spec.k8s_provider_name
                    .clone()
                    .unwrap_or_else(|| cluster_name.clone()),
                "ProviderConfig",
            ),

            aws_enabled: matches!(backend, Backend::Aws),
            aws_region: spec.aws_region.clone().unwrap_or_default(),
            aws_provider: ProviderConfigRef::new(
                spec.aws_provider_name
                    .clone()
                    .unwrap_or_else(|| "default".into()),
                "ProviderConfig",
            ),
            aws_permissions_boundary_arn: spec
                .aws_permissions_boundary_arn
                .clone()
                .unwrap_or_default(),
            aws_role_prefix: spec.aws_role_prefix.clone().unwrap_or_default(),
            aws_tags,

            vault_enabled,
            vault_install,
            vault_namespace,
            vault_release_name,
            vault_server,
            vault_path: spec.vault_path.clone().unwrap_or_else(|| "secret".into()),
            vault_version: spec.vault_version.clone().unwrap_or_else(|| "v2".into()),
            vault_auth_method,
            vault_auth_mount_path: spec
                .vault_auth_mount_path
                .clone()
                .unwrap_or_else(|| "kubernetes".into()),
            vault_auth_role: spec
                .vault_auth_role
                .clone()
                .unwrap_or_else(|| "external-secrets".into()),
            vault_token_secret_name: spec
                .vault_token_secret_name
                .clone()
                .unwrap_or_else(|| "vault-token".into()),
            vault_token_secret_key: spec
                .vault_token_secret_key
                .clone()
                .unwrap_or_else(|| "token".into()),
            vault_token_secret_namespace: spec
                .vault_token_secret_namespace
                .clone()
                .unwrap_or_else(|| namespace.clone()),
            vault_values: if spec.vault_values.is_null() {
                json!({})
            } else {
                spec.vault_values.clone()
            },
            vault_override_all: spec.vault_override_all.clone(),

            secret_store_enabled: spec.secret_store_enabled.unwrap_or(true),
            secret_store_scope: spec.secret_store_scope.unwrap_or_default(),
            secret_store_name: spec
                .secret_store_name
                .clone()
                .unwrap_or_else(|| "default".into()),

            pod_identity_name: format!("{name}-external-secrets"),
            service_account_name: "external-secrets".into(),
            service_account_namespace: namespace,
        }
    }
}

/// Observed slices — mirrors `010-state-status.yaml.gotmpl`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    pub helm_external_secrets: ObservedSlice,
    pub helm_vault: ObservedSlice,
    pub pod_identity: ObservedSlice,
    pub secret_store: ObservedSlice,
}

impl Observed {
    pub fn bootstrap() -> Self {
        Self::default()
    }
}

/// Status fields written back to the XR (ready left for auto-ready in real pipeline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusOut {
    pub ready: bool,
    pub backend: Backend,
    pub secret_store_name: String,
    pub secret_store_scope: SecretStoreScope,
    pub secret_store_ready: bool,
    pub vault: Option<VaultStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultStatus {
    pub release_name: String,
    pub namespace: String,
    pub ready: bool,
}

pub fn compute_status(state: &EffectiveState, obs: &Observed) -> StatusOut {
    StatusOut {
        ready: false, // function-auto-ready owns overall ready in the real pipeline
        backend: state.backend,
        secret_store_name: state.secret_store_name.clone(),
        secret_store_scope: state.secret_store_scope,
        secret_store_ready: obs.secret_store.ready.is_set(),
        vault: state.vault_install.then(|| VaultStatus {
            release_name: state.vault_release_name.clone(),
            namespace: state.vault_namespace.clone(),
            ready: obs.helm_vault.ready.is_set(),
        }),
    }
}

/// Convenience: Ready for multi-resource Usage gates.
pub fn all_ready(slices: &[Ready]) -> Ready {
    Ready(slices.iter().all(|r| r.is_set()))
}
