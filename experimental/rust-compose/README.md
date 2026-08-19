# experimental/rust-compose

**Prototype only.** Re-expresses SecretStack’s go-templating composition as a
Rust layered-gate graph. Not wired into `composition.yaml`.

## Why

SecretStack already has clear layers:

```text
always:     helm-external-secrets
            helm-vault          (vault.install)
            pod-identity        (backend=aws)

under exists(ESO revision>0)
  [+ vault exists if install]
  [+ region/server checks]:
            secret-store

Usages (Ready only):
            usage-pod-identity
            usage-secret-store-helm
            usage-secret-store-vault
```

Gotmpl encodes that as `$state.observed.*.exists` + `if $shouldRender`.
This crate encodes the same graph as:

```rust
d.emit("helm-external-secrets", eso);
d.under_exists(eso_exists & vault_exists_if_needed, |d| {
    d.emit("secret-store", store);
    d.usage_when_ready(store_ready, "usage-secret-store-helm", usage);
});
```

`Exists` and `Ready` are distinct types so dependent MRs cannot accidentally
gate on Ready (the credential-rotation / un-render footgun).

## Map to gotmpl

| File | Rust |
|------|------|
| `functions/render/000-state-init.yaml.gotmpl` | `state::EffectiveState::from_spec` |
| `functions/render/010-state-status.yaml.gotmpl` | `state::Observed` + `compute_status` |
| `200-helm-release-external-secrets.yaml.gotmpl` | root `emit` |
| `201-helm-release-vault.yaml.gotmpl` | root `emit` if `vault.install` |
| `210-aws-pod-identity.yaml.gotmpl` | root `emit` if aws + `usage_when_ready` |
| `230-secret-store.yaml.gotmpl` | `under_exists` + usages |
| `999-status.yaml.gotmpl` | `StatusOut` |

## Run tests

```bash
cd experimental/rust-compose
cargo test
```

Tests cover:

- AWS bootstrap (no SecretStore until ESO exists)
- ESO exists but Ready=False still unlocks SecretStore (upgrade blip)
- Full Ready emits Usages
- Vault install waits for **both** Helm exists signals
- Missing region / `secretStore.enabled=false` blocks store

## Not included (next steps if we productize)

1. Wire gRPC via `crossplane-fn-sdk-unofficial` (or official later)
2. Package as Function xpkg; point composition pipeline at it
3. Optional `compose!` / `under_exists!` macros for thinner call sites
4. Parity render tests against `make render` golden YAML

Production path remains `functions/render/*.gotmpl` until those land.
