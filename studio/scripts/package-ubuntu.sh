#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
test "$(uname -m)" = x86_64 || { echo 'Only x86-64 is supported'; exit 1; }
# Run on Ubuntu 22.04 to preserve compatibility with both supported releases.
cargo build --locked --release --manifest-path ../Cargo.toml
npm ci --no-audit --no-fund
(cd src-tauri/sidecars/agent && npm ci --no-audit --no-fund && npm run build)
mkdir -p src-tauri/runtime src-tauri/ubuntu-resources/demo
cp "$(command -v node)" src-tauri/runtime/node
chmod 755 src-tauri/runtime/node
cp ../demo/*.py src-tauri/ubuntu-resources/demo/
npm run prepare-engine
npm run tauri -- build --bundles deb --config src-tauri/tauri.ubuntu.conf.json
find src-tauri/target/release/bundle/deb -name '*.deb' -exec sha256sum {} \; > src-tauri/target/release/bundle/deb/SHA256SUMS
