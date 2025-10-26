#!/bin/bash
# FreezR Service Installation Test Script

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         FreezR Systemd Service Installation Test         ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""

cd /home/ryazanov/.myBashScripts/freezr

echo "📋 Step 1: Check current service status"
echo "----------------------------------------"
./target/release/process-monitor service-status
echo ""
echo "Press Enter to continue with installation..."
read

echo ""
echo "📝 Step 2: Install FreezR as systemd service"
echo "----------------------------------------"
echo "This will ask for your sudo password..."
sudo ./target/release/process-monitor install-service --yes

echo ""
echo "📊 Step 3: Check service status after installation"
echo "----------------------------------------"
./target/release/process-monitor service-status

echo ""
echo "📋 Step 4: View recent service logs"
echo "----------------------------------------"
sudo journalctl -u freezr -n 20 --no-pager

echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║                   Test Complete!                          ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Useful commands:"
echo "  View logs (real-time):  sudo journalctl -u freezr -f"
echo "  Check status:           sudo systemctl status freezr"
echo "  Restart:                sudo systemctl restart freezr"
echo "  Stop:                   sudo systemctl stop freezr"
echo "  Uninstall:              sudo ./target/release/process-monitor uninstall-service"
