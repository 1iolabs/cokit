#!/bin/bash
set -e

# Build a combined CA bundle from system CAs and any runner-provided internal certs
CA_BUNDLE="/tmp/ca-bundle.crt"
cp /etc/ssl/certs/ca-certificates.crt "$CA_BUNDLE"
if [ -d "${GIT_SSL_CAPATH:-}" ]; then
  cat "$GIT_SSL_CAPATH"/*.crt >> "$CA_BUNDLE" 2>/dev/null || true
  cat "$GIT_SSL_CAPATH"/*.pem >> "$CA_BUNDLE" 2>/dev/null || true
fi
git config --global http.sslCAInfo "$CA_BUNDLE"

# Configure git to use the CI job token for authentication against gitlab.1io.com
git config --global url."https://gitlab-ci-token:${CI_JOB_TOKEN}@gitlab.1io.com/".insteadOf "https://gitlab.1io.com/"

# Forward proxy settings to git if available
if [ -n "${https_proxy:-$HTTPS_PROXY}" ]; then
  git config --global http.proxy "${https_proxy:-$HTTPS_PROXY}"
fi

if [ -n "${no_proxy:-$NO_PROXY}" ]; then
  git config --global http.noProxy "${no_proxy:-$NO_PROXY}"
fi
