# secret-stack

Installs [External Secrets Operator](https://external-secrets.io/) and wires a
SecretStore to either **AWS Secrets Manager** or **HashiCorp Vault**.

**API group:** `hops.ops.com.ai` (platform-neutral; no longer `aws.hops.ops.com.ai`).

## Backend selection

| `spec.backend` | What gets created |
|----------------|-------------------|
| `aws` (default) | ESO Helm + AWS PodIdentity + Secrets Manager SecretStore |
| `vault` | ESO Helm + Vault SecretStore; optional in-cluster Vault Helm (`vault.install`) |

## Usage

### AWS (production EKS)

```yaml
apiVersion: hops.ops.com.ai/v1alpha1
kind: SecretStack
metadata:
  name: external-secrets
  namespace: default
spec:
  clusterName: my-cluster
  backend: aws
  aws:
    region: us-east-1
```

### Vault in-cluster (local / dory)

```yaml
apiVersion: hops.ops.com.ai/v1alpha1
kind: SecretStack
metadata:
  name: external-secrets
  namespace: default
spec:
  clusterName: dory
  backend: vault
  secretStore:
    scope: Cluster
    name: vault
  vault:
    install: true
    # server defaults to http://vault.vault.svc.cluster.local:8200
    auth:
      method: kubernetes
      role: external-secrets
```

### External Vault (token auth)

```yaml
apiVersion: hops.ops.com.ai/v1alpha1
kind: SecretStack
metadata:
  name: external-secrets
  namespace: default
spec:
  clusterName: my-cluster
  backend: vault
  vault:
    install: false
    server: https://vault.example.com:8200
    auth:
      method: token
      tokenSecretRef:
        name: vault-token
        key: token
```

## What Gets Created

| Resource | Condition | Description |
|----------|-----------|-------------|
| `helm.m.crossplane.io/Release` (external-secrets) | Always | ESO chart |
| `helm.m.crossplane.io/Release` (vault) | `backend=vault` + `vault.install` | Official Vault chart (dev-friendly defaults) |
| `aws.hops.ops.com.ai/PodIdentity` | `backend=aws` | IAM role + Pod Identity for SM/SSM/KMS |
| `kubernetes.m.crossplane.io/Object` (SecretStore) | `secretStore.enabled` (default true) | Backend-specific SecretStore / ClusterSecretStore |

## SecretStore Options

| Field | Default | Description |
|-------|---------|-------------|
| `secretStore.enabled` | `true` | Create a SecretStore resource |
| `secretStore.scope` | `Namespaced` | `Namespaced` or `Cluster` |
| `secretStore.name` | `default` | Name of the SecretStore resource |

## Breaking change (from aws-secret-stack)

- API group: `aws.hops.ops.com.ai` → `hops.ops.com.ai`
- Package: `aws-secret-stack` → `secret-stack`
- `spec.aws` is only required when `backend=aws`
- New `spec.backend` and `spec.vault`

## Development

```bash
make render      # render all examples
make validate    # validate rendered output
make test        # run KCL unit tests
```

### Experimental: Rust layered-gate compose

A prototype re-expresses this stack’s render graph (existence gates + Ready
Usages) in Rust. Production still uses `functions/render/*.gotmpl`.

```bash
cd experimental/rust-compose && cargo test
```

See [experimental/rust-compose/README.md](experimental/rust-compose/README.md).

### Local install (source)

```bash
# Confirm with the user before targeting a cluster
hops config install --path xrs/stacks/aws/secret
kubectl apply -f local/secretstack.yaml
```
