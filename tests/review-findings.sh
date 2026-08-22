#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

crossplane_bin="${CROSSPLANE_BIN:-crossplane}"
xrd="apis/secretstacks/definition.yaml"

if "$crossplane_bin" resource validate --help >/dev/null 2>&1; then
  validate_resource() {
    "$crossplane_bin" resource validate "$xrd" "$1"
  }
elif "$crossplane_bin" beta validate --help >/dev/null 2>&1; then
  validate_resource() {
    "$crossplane_bin" beta validate "$1" "$xrd"
  }
else
  echo "Unsupported Crossplane CLI: expected resource validate or beta validate" >&2
  exit 1
fi

expect_invalid() {
  local fixture="$1"
  local expected="$2"
  local output
  local status

  set +e
  output="$(validate_resource "$fixture" 2>&1)"
  status=$?
  set -e

  if [[ $status -eq 0 ]]; then
    echo "Expected schema validation to reject $fixture" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" <<<"$output"; then
    echo "Validation failed for an unexpected reason: $fixture" >&2
    echo "$output" >&2
    exit 1
  fi
}

expect_invalid \
  "tests/fixtures/invalid-aws-missing-region.yaml" \
  "backend=aws requires spec.aws.region"
expect_invalid \
  "tests/fixtures/invalid-vault-missing-server.yaml" \
  "backend=vault with vault.install=false requires spec.vault.server"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

render() {
  local fixture="$1"
  local output="$2"
  local observed="${3:-}"
  if [[ -n "$observed" ]]; then
    up composition render \
      --xrd="$xrd" \
      apis/secretstacks/composition.yaml \
      "$fixture" \
      --observed-resources="$observed" \
      --quiet >"$output"
  else
    up composition render \
      --xrd="$xrd" \
      apis/secretstacks/composition.yaml \
      "$fixture" \
      --quiet >"$output"
  fi
}

render "examples/secretstacks/vault.yaml" "$tmpdir/vault-dev.yaml"
render \
  "examples/secretstacks/vault-external.yaml" \
  "$tmpdir/vault-namespaced.yaml" \
  "examples/test/mocks/observed-resources/standard/steps/1/"
render \
  "tests/fixtures/vault-cluster-token.yaml" \
  "$tmpdir/vault-cluster.yaml" \
  "examples/test/mocks/observed-resources/standard/steps/1/"
render \
  "tests/fixtures/vault-install-production.yaml" \
  "$tmpdir/vault-production.yaml"

ruby -ryaml - "$tmpdir" <<'RUBY'
dir = ARGV.fetch(0)

def documents(path)
  YAML.load_stream(File.read(path)).compact
end

def composed(docs, resource_name)
  docs.find do |doc|
    doc.dig("metadata", "annotations", "crossplane.io/composition-resource-name") == resource_name
  end or raise "missing composed resource #{resource_name}"
end

dev = composed(documents(File.join(dir, "vault-dev.yaml")), "helm-vault")
server = dev.dig("spec", "forProvider", "values", "server")
raise "dev Vault must enable authDelegator" unless server.dig("authDelegator", "enabled") == true
script = server.fetch("postStart").fetch(2)
[
  "vault auth enable",
  "/config",
  "vault policy write",
  "/role/",
].each do |command|
  raise "Vault bootstrap is missing #{command}" unless script.include?(command)
end

namespaced = composed(documents(File.join(dir, "vault-namespaced.yaml")), "secret-store")
namespaced_ref = namespaced.dig(
  "spec", "forProvider", "manifest", "spec", "provider", "vault", "auth", "tokenSecretRef"
)
raise "namespaced SecretStore must omit token namespace" if namespaced_ref.key?("namespace")

cluster = composed(documents(File.join(dir, "vault-cluster.yaml")), "secret-store")
cluster_ref = cluster.dig(
  "spec", "forProvider", "manifest", "spec", "provider", "vault", "auth", "tokenSecretRef"
)
unless cluster_ref["namespace"] == "external-secrets"
  raise "ClusterSecretStore must retain token namespace"
end

production = composed(documents(File.join(dir, "vault-production.yaml")), "helm-vault")
production_server = production.dig("spec", "forProvider", "values", "server")
if production_server.key?("postStart")
  raise "non-dev Vault must not receive the dev root-token bootstrap"
end
RUBY

echo "SecretStack review-finding tests passed"
