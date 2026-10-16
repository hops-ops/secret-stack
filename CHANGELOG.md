### What's changed in v1.0.2

* chore(deps): update helm release external-secrets to v2.12.0 (#32) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* chore(deps): update helm release vault to v0.34.1 (#33) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* chore(deps): update unbounded-tech/workflow-vnext-tag action to v1.22.3 (#34) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* chore(deps): update upbound/action-up action to v1.2.0 (#35) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* chore(deps): update dependency hops-ops/aws-auto-eks-cluster to v2 (#39) (by @renovate[bot])

  e2e test-only: tests/e2etest-secretstacks bootstraps aws-auto-eks-cluster v0.7.0 -> v2.2.1 (same version used by aws-observe/psql/aws-crossplane e2e). Does not change the SecretStack XR render. PR CI incl. e2e green.

* chore(deps): update actions/checkout action to v7 (#38) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* chore(deps): update upbound/action-up-project action to v1.2.0 (#36) (by @renovate[bot])

  Co-authored-by: renovate[bot] <29139614+renovate[bot]@users.noreply.github.com>

* fix(deps): update helm release external-secrets to v2.13.0 (#40) (by @renovate[bot])

  XR-rendered chart pin (200-helm-release-external-secrets). Chart diff 2.12.0->2.13.0 is version/appVersion only; CRDs unchanged. App v2.13.0 tightens Secret validation for target.manifest and v1beta1 templateFrom targets; hops-ops ExternalSecrets use neither. validate/test/e2e green. fix type to cut a release (also ships 2.12.0, automerged earlier as chore).


See full diff: [v1.0.1...v1.0.2](https://github.com/hops-ops/secret-stack/compare/v1.0.1...v1.0.2)
