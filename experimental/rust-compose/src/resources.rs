//! Unstructured resource bodies — mirrors the YAML emitted by gotmpl files.
//! Bodies are JSON Value for the prototype; a full function would emit k8s objects.

use serde_json::{json, Value};

use crate::state::{EffectiveState, SecretStoreScope};

const ESO_CHART_VERSION: &str = "2.2.0";
const VAULT_CHART_VERSION: &str = "0.30.0";

fn labels(state: &EffectiveState) -> Value {
    Value::Object(state.labels.clone())
}

fn management_policies(state: &EffectiveState) -> Value {
    json!(state.management_policies)
}

/// `200-helm-release-external-secrets.yaml.gotmpl`
pub fn helm_external_secrets(state: &EffectiveState) -> Value {
    let values = if let Some(over) = &state.helm_override_all {
        over.clone()
    } else {
        let mut defaults = json!({
            "serviceAccount": {
                "create": true,
                "name": state.service_account_name,
            },
            "resources": {
                "requests": { "cpu": "15m", "memory": "100Mi" },
                "limits": { "cpu": "100m", "memory": "256Mi" }
            },
            "certController": {
                "resources": {
                    "requests": { "cpu": "15m", "memory": "100Mi" },
                    "limits": { "cpu": "100m", "memory": "256Mi" }
                }
            },
            "webhook": {
                "resources": {
                    "requests": { "cpu": "15m", "memory": "100Mi" },
                    "limits": { "cpu": "100m", "memory": "256Mi" }
                }
            }
        });
        merge_objects(&mut defaults, &state.helm_values);
        defaults
    };

    json!({
        "apiVersion": "helm.m.crossplane.io/v1beta1",
        "kind": "Release",
        "metadata": {
            "name": state.helm_release_name,
            "annotations": {
                "crossplane.io/composition-resource-name": "helm-external-secrets"
            },
            "labels": labels(state),
        },
        "spec": {
            "managementPolicies": management_policies(state),
            "forProvider": {
                "chart": {
                    "name": "external-secrets",
                    "repository": "https://charts.external-secrets.io",
                    "version": ESO_CHART_VERSION,
                },
                "namespace": state.helm_namespace,
                "values": values,
            },
            "rollbackLimit": 3,
            "providerConfigRef": {
                "name": state.helm_provider.name,
                "kind": state.helm_provider.kind,
            }
        }
    })
}

/// `201-helm-release-vault.yaml.gotmpl`
pub fn helm_vault(state: &EffectiveState) -> Value {
    let values = if let Some(over) = &state.vault_override_all {
        over.clone()
    } else {
        let mut defaults = json!({
            "global": { "enabled": true, "tlsDisable": true },
            "injector": { "enabled": false },
            "server": {
                "dev": { "enabled": true },
                "standalone": { "enabled": true },
                "dataStorage": { "enabled": false },
                "resources": {
                    "requests": { "cpu": "50m", "memory": "128Mi" },
                    "limits": { "cpu": "500m", "memory": "512Mi" }
                }
            },
            "ui": { "enabled": true }
        });
        merge_objects(&mut defaults, &state.vault_values);
        defaults
    };

    json!({
        "apiVersion": "helm.m.crossplane.io/v1beta1",
        "kind": "Release",
        "metadata": {
            "name": state.vault_release_name,
            "annotations": {
                "crossplane.io/composition-resource-name": "helm-vault"
            },
            "labels": labels(state),
        },
        "spec": {
            "managementPolicies": management_policies(state),
            "forProvider": {
                "chart": {
                    "name": "vault",
                    "repository": "https://helm.releases.hashicorp.com",
                    "version": VAULT_CHART_VERSION,
                },
                "namespace": state.vault_namespace,
                "values": values,
            },
            "rollbackLimit": 3,
            "providerConfigRef": {
                "name": state.helm_provider.name,
                "kind": state.helm_provider.kind,
            }
        }
    })
}

/// `210-aws-pod-identity.yaml.gotmpl` (body only; Usage separate).
pub fn pod_identity(state: &EffectiveState) -> Value {
    let mut spec = json!({
        "managementPolicies": management_policies(state),
        "clusterName": state.cluster_name,
        "region": state.aws_region,
        "providerConfigRef": {
            "name": state.aws_provider.name,
            "kind": state.aws_provider.kind,
        },
        "serviceAccount": {
            "name": state.service_account_name,
            "namespace": state.service_account_namespace,
        },
        "inlinePolicy": [{
            "name": "external-secrets",
            "policy": include_str!("../assets/eso-inline-policy.json")
        }],
        "tags": Value::Object(state.aws_tags.clone()),
    });
    if !state.aws_role_prefix.is_empty() {
        spec["rolePrefix"] = json!(state.aws_role_prefix);
    }
    if !state.aws_permissions_boundary_arn.is_empty() {
        spec["permissionsBoundaryArn"] = json!(state.aws_permissions_boundary_arn);
    }

    json!({
        "apiVersion": "aws.hops.ops.com.ai/v1alpha1",
        "kind": "PodIdentity",
        "metadata": {
            "name": state.pod_identity_name,
            "annotations": {
                "crossplane.io/composition-resource-name": "pod-identity"
            },
            "labels": labels(state),
        },
        "spec": spec,
    })
}

pub fn usage_pod_identity_protects_until_helm_gone(state: &EffectiveState) -> Value {
    json!({
        "apiVersion": "protection.crossplane.io/v1beta1",
        "kind": "Usage",
        "metadata": {
            "name": format!("{}-delete-helm-eso-before-pod-identity", state.name),
            "annotations": {
                "crossplane.io/composition-resource-name": "usage-pod-identity"
            },
            "labels": labels(state),
        },
        "spec": {
            "of": {
                "apiVersion": "aws.hops.ops.com.ai/v1alpha1",
                "kind": "PodIdentity",
                "resourceRef": { "name": state.pod_identity_name }
            },
            "by": {
                "apiVersion": "helm.m.crossplane.io/v1beta1",
                "kind": "Release",
                "resourceRef": { "name": state.helm_release_name }
            },
            "replayDeletion": true
        }
    })
}

/// Provider fragment for SecretStore / ClusterSecretStore.
fn secret_store_provider(state: &EffectiveState) -> Value {
    if state.aws_enabled {
        return json!({
            "aws": {
                "service": "SecretsManager",
                "region": state.aws_region,
            }
        });
    }

    // vault
    let mut vault = json!({
        "server": state.vault_server,
        "path": state.vault_path,
        "version": state.vault_version,
    });

    if state.vault_auth_method == "kubernetes" {
        let mut sa_ref = json!({ "name": state.service_account_name });
        if state.secret_store_scope == SecretStoreScope::Cluster {
            sa_ref["namespace"] = json!(state.service_account_namespace);
        }
        vault["auth"] = json!({
            "kubernetes": {
                "mountPath": state.vault_auth_mount_path,
                "role": state.vault_auth_role,
                "serviceAccountRef": sa_ref,
            }
        });
    } else {
        let mut token_ref = json!({
            "name": state.vault_token_secret_name,
            "key": state.vault_token_secret_key,
        });
        if state.secret_store_scope == SecretStoreScope::Cluster
            || !state.vault_token_secret_namespace.is_empty()
        {
            token_ref["namespace"] = json!(state.vault_token_secret_namespace);
        }
        vault["auth"] = json!({ "tokenSecretRef": token_ref });
    }

    json!({ "vault": vault })
}

/// `230-secret-store.yaml.gotmpl` Object (SecretStore or ClusterSecretStore).
pub fn secret_store_object(state: &EffectiveState) -> Value {
    let provider = secret_store_provider(state);
    let (meta_name, composition_name, manifest) = match state.secret_store_scope {
        SecretStoreScope::Cluster => (
            format!("{}-cluster-secret-store", state.name),
            "secret-store",
            json!({
                "apiVersion": "external-secrets.io/v1",
                "kind": "ClusterSecretStore",
                "metadata": { "name": state.secret_store_name },
                "spec": { "provider": provider },
            }),
        ),
        SecretStoreScope::Namespaced => (
            format!("{}-secret-store", state.name),
            "secret-store",
            json!({
                "apiVersion": "external-secrets.io/v1",
                "kind": "SecretStore",
                "metadata": {
                    "name": state.secret_store_name,
                    "namespace": state.namespace,
                },
                "spec": { "provider": provider },
            }),
        ),
    };

    json!({
        "apiVersion": "kubernetes.m.crossplane.io/v1alpha1",
        "kind": "Object",
        "metadata": {
            "name": meta_name,
            "annotations": {
                "crossplane.io/composition-resource-name": composition_name
            },
            "labels": labels(state),
        },
        "spec": {
            "managementPolicies": management_policies(state),
            "forProvider": { "manifest": manifest },
            "providerConfigRef": {
                "name": state.k8s_provider.name,
                "kind": state.k8s_provider.kind,
            }
        }
    })
}

fn secret_store_object_name(state: &EffectiveState) -> String {
    match state.secret_store_scope {
        SecretStoreScope::Cluster => format!("{}-cluster-secret-store", state.name),
        SecretStoreScope::Namespaced => format!("{}-secret-store", state.name),
    }
}

pub fn usage_secret_store_before_eso_helm(state: &EffectiveState) -> Value {
    json!({
        "apiVersion": "protection.crossplane.io/v1beta1",
        "kind": "Usage",
        "metadata": {
            "name": format!("{}-delete-secret-store-before-helm", state.name),
            "annotations": {
                "crossplane.io/composition-resource-name": "usage-secret-store-helm"
            },
            "labels": labels(state),
        },
        "spec": {
            "replayDeletion": true,
            "of": {
                "apiVersion": "helm.m.crossplane.io/v1beta1",
                "kind": "Release",
                "resourceRef": { "name": state.helm_release_name }
            },
            "by": {
                "apiVersion": "kubernetes.m.crossplane.io/v1alpha1",
                "kind": "Object",
                "resourceRef": { "name": secret_store_object_name(state) }
            }
        }
    })
}

pub fn usage_secret_store_before_vault(state: &EffectiveState) -> Value {
    json!({
        "apiVersion": "protection.crossplane.io/v1beta1",
        "kind": "Usage",
        "metadata": {
            "name": format!("{}-delete-secret-store-before-vault", state.name),
            "annotations": {
                "crossplane.io/composition-resource-name": "usage-secret-store-vault"
            },
            "labels": labels(state),
        },
        "spec": {
            "replayDeletion": true,
            "of": {
                "apiVersion": "helm.m.crossplane.io/v1beta1",
                "kind": "Release",
                "resourceRef": { "name": state.vault_release_name }
            },
            "by": {
                "apiVersion": "kubernetes.m.crossplane.io/v1alpha1",
                "kind": "Object",
                "resourceRef": { "name": secret_store_object_name(state) }
            }
        }
    })
}

/// Shallow-ish recursive merge of object maps (gotmpl mergeOverwrite-ish for objects).
fn merge_objects(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(k) {
                    Some(existing) => merge_objects(existing, v),
                    None => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}
