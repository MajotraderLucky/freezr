# FreezR Systemd Integration

## Overview

This guide explains how to run FreezR as a systemd service for automatic startup and management.

## Benefits of Systemd Integration

- ✅ **Auto-start on boot** - FreezR starts automatically after system reboot
- ✅ **Auto-restart on failure** - Automatic recovery if daemon crashes
- ✅ **Centralized logging** - Logs accessible via `journalctl`
- ✅ **Resource limits** - CPU/memory limits for the daemon itself
- ✅ **Standard management** - Use familiar `systemctl` commands

## Installation Steps

### 1. Build Release Binary

```bash
cd /home/ryazanov/.myBashScripts/freezr
cargo build --release
```

### 2. Install Binary System-wide

```bash
# Copy binary to system location
sudo cp target/release/freezr-daemon /usr/local/bin/

# Verify
freezr-daemon --version
```

### 3. Create Configuration Directory

```bash
# Create config directory
sudo mkdir -p /etc/freezr

# Generate default config
sudo freezr-daemon generate-config --output /etc/freezr/config.toml

# Edit config as needed
sudo nano /etc/freezr/config.toml
```

### 4. Create Log Directory

```bash
# Create log directory
sudo mkdir -p /var/log/freezr

# Set permissions
sudo chown root:root /var/log/freezr
sudo chmod 755 /var/log/freezr
```

### 5. Update Configuration for System Paths

Edit `/etc/freezr/config.toml`:

```toml
[logging]
log_dir = "/var/log/freezr"
kesl_log = "kesl-monitor.log"
node_log = "node-monitor.log"
actions_log = "actions.log"
max_file_size_mb = 10
rotate_count = 5
```

### 6. Create Systemd Service File

```bash
sudo tee /etc/systemd/system/freezr.service > /dev/null << 'EOF'
[Unit]
Description=FreezR System Resource Guardian
Documentation=file:///home/ryazanov/.myBashScripts/freezr/docs/
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/freezr-daemon --config /etc/freezr/config.toml watch
Restart=always
RestartSec=10
User=root

# Working directory for relative paths
WorkingDirectory=/var/lib/freezr

# Resource limits for FreezR daemon itself
CPUQuota=5%
MemoryMax=50M

# Security hardening
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF
```

### 7. Create Working Directory

```bash
sudo mkdir -p /var/lib/freezr
sudo chown root:root /var/lib/freezr
```

### 8. Enable and Start Service

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable freezr

# Start service now
sudo systemctl start freezr

# Check status
sudo systemctl status freezr
```

## Service Management

### Check Status

```bash
# View service status
sudo systemctl status freezr

# Expected output:
# ● freezr.service - FreezR System Resource Guardian
#    Loaded: loaded (/etc/systemd/system/freezr.service; enabled)
#    Active: active (running) since ...
#    Main PID: 12345
```

### Start/Stop/Restart

```bash
# Start service
sudo systemctl start freezr

# Stop service
sudo systemctl stop freezr

# Restart service
sudo systemctl restart freezr

# Reload configuration (if supported)
sudo systemctl reload freezr
```

### Enable/Disable Auto-start

```bash
# Enable auto-start on boot
sudo systemctl enable freezr

# Disable auto-start
sudo systemctl disable freezr

# Check if enabled
sudo systemctl is-enabled freezr
```

### View Logs

```bash
# View all logs
sudo journalctl -u freezr

# Follow logs in real-time
sudo journalctl -u freezr -f

# View last 100 lines
sudo journalctl -u freezr -n 100

# View logs since boot
sudo journalctl -u freezr -b

# View logs for specific time range
sudo journalctl -u freezr --since "2025-10-25 10:00" --until "2025-10-25 12:00"

# View logs with priority level
sudo journalctl -u freezr -p err  # Errors only
sudo journalctl -u freezr -p warning  # Warnings and above
```

### Check File Logs

```bash
# File logs (if configured)
tail -f /var/log/freezr/freezr-daemon.log.$(date +%Y-%m-%d)

# Search for specific events
grep "violation" /var/log/freezr/*.log
grep "restart" /var/log/freezr/*.log
grep "killed" /var/log/freezr/*.log
```

## Service File Options

### Basic Configuration

```ini
[Unit]
Description=FreezR System Resource Guardian
After=network.target
```

- `Description`: Human-readable service description
- `After`: Start after network is available

### Service Execution

```ini
[Service]
Type=simple
ExecStart=/usr/local/bin/freezr-daemon --config /etc/freezr/config.toml watch
```

- `Type=simple`: Service runs in foreground
- `ExecStart`: Command to execute

### Auto-restart

```ini
Restart=always
RestartSec=10
```

- `Restart=always`: Restart on any exit (success or failure)
- `RestartSec=10`: Wait 10 seconds before restart

### Resource Limits

```ini
CPUQuota=5%
MemoryMax=50M
```

- `CPUQuota=5%`: Limit daemon to 5% CPU
- `MemoryMax=50M`: Limit daemon to 50MB RAM

### Security Hardening

```ini
NoNewPrivileges=true
PrivateTmp=true
ReadOnlyPaths=/etc
```

- `NoNewPrivileges`: Prevent privilege escalation
- `PrivateTmp`: Private /tmp directory
- `ReadOnlyPaths`: Make paths read-only

## Advanced Configuration

### Multiple Instances

Run multiple FreezR instances with different configs:

```bash
# Create instance-specific config
sudo cp /etc/freezr/config.toml /etc/freezr/config-kesl.toml
sudo cp /etc/freezr/config.toml /etc/freezr/config-node.toml

# Edit configs to enable only specific monitors
# config-kesl.toml: node.enabled = false
# config-node.toml: kesl.enabled = false

# Create service instances
sudo cp /etc/systemd/system/freezr.service /etc/systemd/system/freezr-kesl.service
sudo cp /etc/systemd/system/freezr.service /etc/systemd/system/freezr-node.service

# Edit ExecStart in each service file
# freezr-kesl.service: --config /etc/freezr/config-kesl.toml
# freezr-node.service: --config /etc/freezr/config-node.toml

# Start both
sudo systemctl daemon-reload
sudo systemctl enable freezr-kesl freezr-node
sudo systemctl start freezr-kesl freezr-node
```

### Custom Environment Variables

```ini
[Service]
Environment="RUST_LOG=debug"
Environment="RUST_BACKTRACE=1"
```

### Dependency Management

```ini
[Unit]
Requires=kesl.service
After=kesl.service
```

Starts FreezR only after KESL service is running.

### Pre/Post Actions

```ini
[Service]
ExecStartPre=/usr/local/bin/freezr-preflight-check.sh
ExecStartPost=/usr/bin/logger "FreezR started"
ExecStopPost=/usr/bin/logger "FreezR stopped"
```

## Monitoring Service Health

### Check if Service is Running

```bash
# Quick check
sudo systemctl is-active freezr

# Detailed status
sudo systemctl status freezr
```

### Check for Crashes

```bash
# View failed service runs
sudo journalctl -u freezr --since today | grep -i error

# Check restart count
sudo systemctl show freezr | grep NRestarts
```

### Performance Monitoring

```bash
# CPU/Memory usage
systemd-cgtop -1 | grep freezr

# Detailed resource usage
sudo systemctl status freezr | grep -E "Memory:|CPU:"
```

## Troubleshooting

### Service Fails to Start

**Check logs:**
```bash
sudo journalctl -u freezr -n 50
```

**Common causes:**
- Configuration error: Check `/etc/freezr/config.toml`
- Binary not found: Verify `/usr/local/bin/freezr-daemon` exists
- Permission denied: Service must run as root

### Service Restarts Frequently

**Check restart count:**
```bash
sudo systemctl show freezr | grep NRestarts
```

**Possible causes:**
- Daemon crashing: Check logs for panics/errors
- Resource limits too low: Increase CPUQuota/MemoryMax
- Configuration issue: Validate config file

### Logs Not Appearing

**Check log directory permissions:**
```bash
ls -la /var/log/freezr
```

**Ensure directory is writable:**
```bash
sudo chmod 755 /var/log/freezr
```

### High Memory Usage

**Monitor memory:**
```bash
sudo systemctl status freezr | grep Memory
```

**Adjust limits if needed:**
```bash
# Edit service file
sudo systemctl edit freezr

# Add override:
[Service]
MemoryMax=100M
```

## Uninstallation

### Stop and Disable Service

```bash
# Stop service
sudo systemctl stop freezr

# Disable auto-start
sudo systemctl disable freezr

# Remove service file
sudo rm /etc/systemd/system/freezr.service

# Reload systemd
sudo systemctl daemon-reload
```

### Remove Files

```bash
# Remove binary
sudo rm /usr/local/bin/freezr-daemon

# Remove configuration
sudo rm -rf /etc/freezr

# Remove logs
sudo rm -rf /var/log/freezr

# Remove working directory
sudo rm -rf /var/lib/freezr
```

## Best Practices

1. **Always use `daemon-reload`** after editing service files
2. **Monitor logs regularly** for violations and errors
3. **Set resource limits** to prevent daemon from consuming too many resources
4. **Use `Restart=always`** for reliability
5. **Enable service** for auto-start on boot
6. **Test configuration** before deploying to production
7. **Keep logs rotated** to prevent disk space issues

## Example: Complete Production Setup

```bash
#!/bin/bash
set -e

echo "Installing FreezR as systemd service..."

# Build and install binary
cd /home/ryazanov/.myBashScripts/freezr
cargo build --release
sudo cp target/release/freezr-daemon /usr/local/bin/

# Create directories
sudo mkdir -p /etc/freezr /var/log/freezr /var/lib/freezr

# Generate config
sudo freezr-daemon generate-config --output /etc/freezr/config.toml

# Update log path in config
sudo sed -i 's|log_dir = "./logs"|log_dir = "/var/log/freezr"|' /etc/freezr/config.toml

# Create service file
sudo tee /etc/systemd/system/freezr.service > /dev/null << 'EOF'
[Unit]
Description=FreezR System Resource Guardian
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/freezr-daemon --config /etc/freezr/config.toml watch
Restart=always
RestartSec=10
User=root
WorkingDirectory=/var/lib/freezr
CPUQuota=5%
MemoryMax=50M
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable freezr
sudo systemctl start freezr

# Show status
sudo systemctl status freezr

echo "FreezR installation complete!"
echo "View logs with: sudo journalctl -u freezr -f"
```

Save as `install-freezr-service.sh` and run with `sudo bash install-freezr-service.sh`.
