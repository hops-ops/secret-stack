### What's changed in v1.0.0

* feat: platform SecretStack with AWS and Vault backends (#28) (by @patrickleet)

  BREAKING CHANGE: * feat: platform SecretStack with AWS and Vault backends

  Break the API group to hops.ops.com.ai and select secrets backend via
  spec.backend (aws|vault). Gate PodIdentity and SM stores on aws; add optional
  Vault Helm install plus Vault SecretStore. Gate dependents on sticky helm
  revision existence rather than Ready. Dogfooded locally with hops config
  install --path against dory (ESO + Vault Releases Ready).

  * feat: prototype typed SecretStack composition gates

  * fix: restore local SecretStack validation

  * chore: remove accidental Rust composition prototype

  Implements [[tasks/remove-secretstack-rust-prototype]]

  * fix: validate and bootstrap SecretStack backends

  Implements [[tasks/address-secretstack-review-findings]]

  * fix: harden review workflow checkout

  Implements [[tasks/address-secretstack-review-findings]]

  * fix: compute SecretStack readiness

  Implements [[tasks/fix-secretstack-status-ready]]

  * fix: preserve disabled SecretStore setting

  Implements [[tasks/fix-secretstack-status-ready]]


See full diff: [v0.20.0...v1.0.0](https://github.com/hops-ops/secret-stack/compare/v0.20.0...v1.0.0)
