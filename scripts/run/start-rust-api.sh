#!/usr/bin/env bash
# Start only the KFCode API server.
# Run in a separate terminal to see API logs. Then run start-web-app.sh in another terminal.

set -e
cd "$(dirname "$0")/../.."

RUST_PORT="${RUST_PORT:-4096}"

echo "Building kfcode-cli (ensures /path returns worktree for E2E)..."
cargo build -p kfcode-cli

echo "Starting Rust API server on http://127.0.0.1:${RUST_PORT}"
echo "Logs below. Use Ctrl+C to stop."
echo "---"
./target/debug/kfcode serve --port "$RUST_PORT" --hostname 127.0.0.1
