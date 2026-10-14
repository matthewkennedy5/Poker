#!/bin/bash
# Run this on a fresh Ubuntu 24.04 Hetzner VPS as root.
# Usage: ssh root@<IP> 'bash -s' < setup-server.sh

set -e

echo "=== System update ==="
apt update && apt upgrade -y

echo "=== Firewall ==="
ufw allow OpenSSH
ufw allow 80
ufw allow 443
yes | ufw enable

echo "=== SSH hardening ==="
sed -i 's/PermitRootLogin yes/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config
systemctl restart ssh

echo "=== Build tools ==="
apt install -y build-essential

echo "=== Install fish shell ==="
apt install -y fish

echo "=== Install Rust ==="
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
fish -c 'fish_add_path $HOME/.cargo/bin'

echo "=== Install uv ==="
snap install astral-uv --classic

echo "=== Install Caddy ==="
apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | tee /etc/apt/sources.list.d/caddy-stable.list
apt update && apt install caddy -y

echo "=== Cargo config (target-cpu=native) ==="
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << 'EOF'

[build]
rustflags = ["-C", "target-cpu=native"]
EOF

echo "=== Create project directory ==="
mkdir -p /root/Poker/rust/products
mkdir -p /root/Poker/gui/build

echo "=== Install Caddyfile ==="
cat > /etc/caddy/Caddyfile << 'EOF'
optimus-poker.com {
    handle /api/* {
        reverse_proxy localhost:8080
    }
    handle {
        root * /root/Poker/gui/build
        file_server
        try_files {path} /index.html
    }
}
EOF
systemctl restart caddy

echo "=== Install poker systemd service ==="
cat > /etc/systemd/system/poker.service << 'EOF'
[Unit]
Description=Poker Bot Server
After=network.target

[Service]
Type=simple
WorkingDirectory=/root/Poker/rust
ExecStart=/root/Poker/rust/target/release/server
Restart=always
RestartSec=1

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable poker

echo "=== Done! ==="
echo "Now from your local machine run:"
echo ""
echo "  # 1. Copy readonly product files from old server"
echo "  scp poker-old:'~/Poker/rust/products/readonly_*' /tmp/poker_products/"
echo "  scp /tmp/poker_products/* root@<NEW_IP>:/root/Poker/rust/products/"
echo ""
echo "  # 2. Rsync source code and GUI build"
echo "  rsync -avz --exclude target/ --exclude node_modules/ --exclude .git/ ~/Desktop/Code/Poker/ root@<NEW_IP>:/root/Poker/"
echo ""
echo "  # 3. SSH in and build + start"
echo "  ssh root@<NEW_IP>"
echo "  cd /root/Poker/rust && ~/.cargo/bin/cargo build --release --bin server"
echo "  sudo systemctl start poker"
echo ""
echo "  # 4. Point DNS (optimus-poker.com) to the new server IP"
