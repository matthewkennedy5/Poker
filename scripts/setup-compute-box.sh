#!/bin/bash
# Compute-box variant of setup-server.sh: no Caddy, no web service.
set -e
export DEBIAN_FRONTEND=noninteractive
echo "=== System update ==="
apt-get update -q && apt-get upgrade -y -q
echo "=== Firewall ==="
ufw allow OpenSSH
yes | ufw enable
echo "=== SSH hardening ==="
sed -i 's/^#\?PermitRootLogin .*/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config
systemctl restart ssh
echo "=== Packages ==="
apt-get install -y -q build-essential pkg-config git rsync htop tmux fish python3-requests python3-numpy
echo "=== Rust ==="
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q
source ~/.cargo/env
mkdir -p ~/.cargo
cat > ~/.cargo/config.toml <<'CFG'
[build]
rustflags = ["-C", "target-cpu=native"]
CFG
mkdir -p /root/Poker/rust/products /root/Poker/results
echo "=== Done ==="
rustc --version
