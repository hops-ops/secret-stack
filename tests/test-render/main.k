import models.io.upbound.dev.meta.v1alpha1 as metav1alpha1
import models.k8s.apimachinery.pkg.apis.meta.v1 as metav1

# ==============================================================================
# Unit tests for platform SecretStack XRD
# ==============================================================================

_readyConditions = [
    {type = "Ready", status = "True"}
    {type = "Synced", status = "True"}
]

_observedHelmExternalSecrets = {
    apiVersion = "helm.m.crossplane.io/v1beta1"
    kind = "Release"
    metadata = {
        name = "external-secrets"
        annotations = {"crossplane.io/composition-resource-name" = "helm-external-secrets"}
    }
    status = {
        atProvider = {revision = 1, state = "deployed"}
        conditions = _readyConditions
    }
}

_observedHelmVault = {
    apiVersion = "helm.m.crossplane.io/v1beta1"
    kind = "Release"
    metadata = {
        name = "vault"
        annotations = {"crossplane.io/composition-resource-name" = "helm-vault"}
    }
    status = {
        atProvider = {revision = 1, state = "deployed"}
        conditions = _readyConditions
    }
}

_observedPodIdentity = {
    apiVersion = "aws.hops.ops.com.ai/v1alpha1"
    kind = "PodIdentity"
    metadata = {
        name = "test-external-secrets"
        annotations = {"crossplane.io/composition-resource-name" = "pod-identity"}
    }
    status.conditions = _readyConditions
}

_observedSecretStore = {
    apiVersion = "kubernetes.m.crossplane.io/v1alpha1"
    kind = "Object"
    metadata = {
        name = "test-secret-store"
        annotations = {"crossplane.io/composition-resource-name" = "secret-store"}
    }
    status.conditions = _readyConditions
}

_observedUsageSecretStoreHelm = {
    apiVersion = "protection.crossplane.io/v1beta1"
    kind = "Usage"
    metadata = {
        name = "test-delete-secret-store-before-helm"
        annotations = {"crossplane.io/composition-resource-name" = "usage-secret-store-helm"}
    }
    status.conditions = _readyConditions
}

_observedUsageSecretStoreVault = {
    apiVersion = "protection.crossplane.io/v1beta1"
    kind = "Usage"
    metadata = {
        name = "test-delete-secret-store-before-vault"
        annotations = {"crossplane.io/composition-resource-name" = "usage-secret-store-vault"}
    }
    status.conditions = _readyConditions
}

_observedUsagePodIdentity = {
    apiVersion = "protection.crossplane.io/v1beta1"
    kind = "Usage"
    metadata = {
        name = "test-delete-helm-eso-before-pod-identity"
        annotations = {"crossplane.io/composition-resource-name" = "usage-pod-identity"}
    }
    status.conditions = _readyConditions
}

_items = [
    # ==========================================================================
    # Test 1: AWS minimal renders Helm Release and Pod Identity
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "aws-minimal-renders-helm-and-pod-identity"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    clusterName = "my-cluster"
                    backend = "aws"
                    aws = {
                        region = "us-east-1"
                    }
                }
            }
            assertResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata.name = "external-secrets"
                    spec.forProvider = {
                        chart.name = "external-secrets"
                    }
                }
                {
                    apiVersion = "aws.hops.ops.com.ai/v1alpha1"
                    kind = "PodIdentity"
                    metadata.name = "test-external-secrets"
                    spec = {
                        clusterName = "my-cluster"
                        region = "us-east-1"
                        serviceAccount = {
                            name = "external-secrets"
                            namespace = "external-secrets"
                        }
                    }
                }
            ]
        }
    }

    # ==========================================================================
    # Test 2: Vault backend skips PodIdentity, installs Vault when asked
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "vault-install-no-pod-identity"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    clusterName = "dory"
                    backend = "vault"
                    vault = {
                        install = True
                    }
                }
            }
            assertResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata.name = "external-secrets"
                }
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata.name = "vault"
                    spec.forProvider = {
                        chart.name = "vault"
                        namespace = "vault"
                        values.server = {
                            authDelegator.enabled = True
                            extraEnvironmentVars = {
                                HOPS_VAULT_AUTH_MOUNT_PATH = "kubernetes"
                                HOPS_VAULT_AUTH_ROLE = "external-secrets"
                                HOPS_VAULT_ESO_NAMESPACE = "external-secrets"
                                HOPS_VAULT_ESO_SERVICE_ACCOUNT = "external-secrets"
                                HOPS_VAULT_PATH = "secret"
                            }
                        }
                    }
                }
            ]
        }
    }

    # ==========================================================================
    # Test 3: Custom namespace propagates (AWS)
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "custom-namespace-propagates"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    clusterName = "prod-cluster"
                    backend = "aws"
                    namespace = "secrets-system"
                    aws = {
                        region = "us-west-2"
                        rolePrefix = "prod-"
                    }
                }
            }
            assertResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata.name = "external-secrets"
                    spec.forProvider.namespace = "secrets-system"
                }
                {
                    apiVersion = "aws.hops.ops.com.ai/v1alpha1"
                    kind = "PodIdentity"
                    metadata.name = "test-external-secrets"
                    spec = {
                        rolePrefix = "prod-"
                        serviceAccount = {
                            namespace = "secrets-system"
                        }
                    }
                }
            ]
        }
    }

    # ==========================================================================
    # Test 4: AWS namespaced SecretStore when ESO exists (revision > 0)
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "aws-secret-store-default-namespaced"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    clusterName = "my-cluster"
                    backend = "aws"
                    aws.region = "us-east-1"
                }
            }
            observedResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata = {
                        name = "external-secrets"
                        annotations = {"crossplane.io/composition-resource-name" = "helm-external-secrets"}
                    }
                    status = {
                        atProvider = {
                            revision = 1
                            state = "deployed"
                        }
                        conditions = [
                            {type = "Ready", status = "True"}
                            {type = "Synced", status = "True"}
                        ]
                    }
                }
            ]
            assertResources = [
                {
                    apiVersion = "kubernetes.m.crossplane.io/v1alpha1"
                    kind = "Object"
                    metadata.name = "test-secret-store"
                    spec.forProvider.manifest = {
                        apiVersion = "external-secrets.io/v1"
                        kind = "SecretStore"
                        metadata = {
                            name = "default"
                            namespace = "external-secrets"
                        }
                        spec.provider.aws = {
                            service = "SecretsManager"
                            region = "us-east-1"
                        }
                    }
                }
            ]
        }
    }

    # ==========================================================================
    # Test 5: Vault ClusterSecretStore when ESO + Vault exist (revision > 0)
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "vault-cluster-secret-store"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    clusterName = "dory"
                    backend = "vault"
                    secretStore = {
                        scope = "Cluster"
                        name = "vault"
                    }
                    vault = {
                        install = True
                        auth = {
                            method = "kubernetes"
                            role = "external-secrets"
                        }
                    }
                }
            }
            observedResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata = {
                        name = "external-secrets"
                        annotations = {"crossplane.io/composition-resource-name" = "helm-external-secrets"}
                    }
                    status = {
                        atProvider = {
                            revision = 1
                            state = "deployed"
                        }
                        conditions = [
                            {type = "Ready", status = "True"}
                            {type = "Synced", status = "True"}
                        ]
                    }
                }
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata = {
                        name = "vault"
                        annotations = {"crossplane.io/composition-resource-name" = "helm-vault"}
                    }
                    status = {
                        atProvider = {
                            revision = 1
                            state = "deployed"
                        }
                        conditions = [
                            {type = "Ready", status = "True"}
                            {type = "Synced", status = "True"}
                        ]
                    }
                }
            ]
            assertResources = [
                {
                    apiVersion = "kubernetes.m.crossplane.io/v1alpha1"
                    kind = "Object"
                    metadata.name = "test-cluster-secret-store"
                    spec.forProvider.manifest = {
                        apiVersion = "external-secrets.io/v1"
                        kind = "ClusterSecretStore"
                        metadata.name = "vault"
                        spec.provider.vault = {
                            server = "http://vault.vault.svc.cluster.local:8200"
                            path = "secret"
                            version = "v2"
                        }
                    }
                }
            ]
        }
    }

    # ==========================================================================
    # Test 6: SecretStore disabled renders nothing store-related
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "secret-store-disabled"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "no-store"
                spec = {
                    clusterName = "my-cluster"
                    backend = "aws"
                    secretStore.enabled = False
                    aws.region = "us-east-1"
                }
            }
            observedResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata = {
                        name = "external-secrets"
                        annotations = {"crossplane.io/composition-resource-name" = "helm-external-secrets"}
                    }
                    status = {
                        atProvider = {
                            revision = 1
                            state = "deployed"
                        }
                        conditions = [
                            {type = "Ready", status = "True"}
                            {type = "Synced", status = "True"}
                        ]
                    }
                }
            ]
            assertResources = [
                {
                    apiVersion = "helm.m.crossplane.io/v1beta1"
                    kind = "Release"
                    metadata.name = "external-secrets"
                }
                {
                    apiVersion = "aws.hops.ops.com.ai/v1alpha1"
                    kind = "PodIdentity"
                    metadata.name = "no-store-external-secrets"
                }
            ]
        }
    }

    # ==========================================================================
    # Test 7: Readiness starts false without observed resources
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-false-before-observation"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "vault"
                    vault.install = True
                }
            }
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = False
            }]
        }
    }

    # ==========================================================================
    # Test 8: Installed Vault is ready only after all resources and Usages
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-vault-install"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "vault"
                    vault.install = True
                }
            }
            observedResources = [
                _observedHelmExternalSecrets
                _observedHelmVault
                _observedSecretStore
                _observedUsageSecretStoreHelm
                _observedUsageSecretStoreVault
            ]
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = True
            }]
        }
    }

    # ==========================================================================
    # Test 9: Missing rendered Usage keeps installed Vault unready
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-vault-waits-for-usage"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "vault"
                    vault.install = True
                }
            }
            observedResources = [
                _observedHelmExternalSecrets
                _observedHelmVault
                _observedSecretStore
            ]
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = False
            }]
        }
    }

    # ==========================================================================
    # Test 10: External Vault does not wait for a Vault Helm release
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-vault-external"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "vault"
                    vault = {
                        install = False
                        server = "https://vault.example.com"
                    }
                }
            }
            observedResources = [
                _observedHelmExternalSecrets
                _observedSecretStore
                _observedUsageSecretStoreHelm
            ]
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = True
            }]
        }
    }

    # ==========================================================================
    # Test 11: AWS readiness includes PodIdentity and its Usage
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-aws"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "aws"
                    aws.region = "us-east-1"
                }
            }
            observedResources = [
                _observedHelmExternalSecrets
                _observedPodIdentity
                _observedSecretStore
                _observedUsageSecretStoreHelm
                _observedUsagePodIdentity
            ]
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = True
            }]
        }
    }

    # ==========================================================================
    # Test 12: Disabled SecretStore is excluded from AWS readiness
    # ==========================================================================
    metav1alpha1.CompositionTest {
        metadata.name = "status-ready-aws-without-secret-store"
        spec = {
            compositionPath = "apis/secretstacks/composition.yaml"
            xrdPath = "apis/secretstacks/definition.yaml"
            timeoutSeconds = 60
            validate = False
            xr = {
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                spec = {
                    backend = "aws"
                    secretStore.enabled = False
                    aws.region = "us-east-1"
                }
            }
            observedResources = [
                _observedHelmExternalSecrets
                _observedPodIdentity
                _observedUsagePodIdentity
            ]
            assertResources = [{
                apiVersion = "hops.ops.com.ai/v1alpha1"
                kind = "SecretStack"
                metadata.name = "test"
                status.ready = True
            }]
        }
    }
]

items = _items
