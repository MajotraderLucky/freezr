# FreezR Usage Guide

## Quick Start

### 1. Generate Configuration File

```bash
# Generate default configuration
./target/release/freezr-daemon generate-config --output config.toml

# Or use the example configuration
cp config/examples/config.toml /etc/freezr/config.toml
```

### 2. Run Single Check (Monitor Mode)

```bash
# Monitor once with default config
./target/release/freezr-daemon monitor

# Monitor with custom config
./target/release/freezr-daemon --config config.toml monitor
```

**Output:**
```
INFO FreezR Daemon v0.1.0 starting...
INFO Configuration loaded and validated successfully
INFO Running single monitoring check...
INFO Node.js monitoring enabled: CPU threshold 80.0%, auto-kill: true
INFO KESL process: PID 122326, CPU 0.0%, Memory 2MB
INFO === Monitoring Status ===
INFO Total checks: 1
INFO CPU violations: 0 (current session: 0)
INFO Memory violations: 0 (current session: 0)
INFO Total restarts: 0
INFO Total kills: 0
INFO Single check completed
```

### 3. Run Continuous Monitoring (Watch Mode)

```bash
# Start continuous monitoring
./target/release/freezr-daemon watch

# Or with custom config
./target/release/freezr-daemon --config config.toml watch
```

This will:
- Check processes every 3 seconds (configurable)
- Track CPU/memory violations
- Automatically restart KESL service after 3 violations
- Automatically kill Node.js processes with CPU >80%
- Log all actions to `./logs/freezr-daemon.log`

**Press Ctrl+C to stop.**

### 4. Force Restart KESL Service

```bash
# Force restart KESL with resource limits
./target/release/freezr-daemon force-restart
```

## Configuration

Default configuration file (`config.toml`):

```toml
[kesl]
# CPU threshold in percent (matches CPUQuota=30%)
cpu_threshold = 30.0

# Memory threshold in MB (warning level, above hard limit of 512MB)
memory_threshold_mb = 600

# Maximum violations before automatic service restart
max_violations = 3

# Systemd service name to manage
service_name = "kesl"

# Enable KESL monitoring
enabled = true

[node]
# CPU threshold for Node.js processes (80%+ indicates hung process)
cpu_threshold = 80.0

# Enable Node.js process monitoring
enabled = true

# Automatically kill high-CPU Node.js processes
auto_kill = true

# Require confirmation before killing (only in interactive mode)
confirm_kill = false

[logging]
# Log directory path (relative to executable or absolute)
log_dir = "./logs"

# Individual log file names
kesl_log = "kesl-monitor.log"
node_log = "node-monitor.log"
actions_log = "actions.log"

# Maximum log file size in MB before rotation
max_file_size_mb = 10

# Number of rotated log files to keep
rotate_count = 5

[monitoring]
# Check interval in seconds (how often to scan processes)
check_interval_secs = 3

# Minimum interval between service restarts in seconds
# Prevents restart loops
min_restart_interval_secs = 100
```

## How It Works

### KESL Monitoring

1. **CPU Measurement**: 3 samples averaged over 2 seconds for accuracy
2. **Memory Tracking**: RSS (Resident Set Size) in MB
3. **Violation Counter**: Increments when thresholds exceeded
4. **Auto-recovery**: Counters reset when values return to normal
5. **Restart Protection**: Max 3 violations trigger restart, minimum 100s between restarts

### Node.js Monitoring

1. **Process Detection**: Finds all processes with command ending in "node"
2. **CPU Threshold**: Default 80% (configurable)
3. **Auto-kill**: Immediate termination via SIGTERM → SIGKILL
4. **No Violations**: Node processes are killed immediately, no counter tracking

## Logs

All logs are written to `./logs/freezr-daemon.log.YYYY-MM-DD`:

```bash
# View today's logs
tail -f logs/freezr-daemon.log.$(date +%Y-%m-%d)

# Search for violations
grep "violation" logs/freezr-daemon.log.*

# Search for restarts
grep "restart" logs/freezr-daemon.log.*
```

## Systemd Integration

Create `/etc/systemd/system/freezr.service`:

```ini
[Unit]
Description=FreezR System Resource Guardian
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/freezr-daemon watch
Restart=always
RestartSec=10
User=root

# Resource limits for the daemon itself
CPUQuota=5%
MemoryMax=50M

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable freezr
sudo systemctl start freezr
sudo systemctl status freezr
```

## CLI Commands

### Monitor (Single Check)
```bash
freezr-daemon monitor
```
Performs one monitoring check and displays statistics.

### Watch (Continuous)
```bash
freezr-daemon watch
```
Runs continuous monitoring loop with automatic actions.

### Force Restart
```bash
freezr-daemon force-restart
```
Immediately restarts KESL service with daemon-reload.

### Generate Config
```bash
freezr-daemon generate-config --output /path/to/config.toml
```
Creates default configuration file.

### Help
```bash
freezr-daemon --help
freezr-daemon <command> --help
```

## Examples

### Basic Monitoring
```bash
# Generate config
./target/release/freezr-daemon generate-config

# Run single check
./target/release/freezr-daemon monitor
```

### Production Setup
```bash
# Install binary
sudo cp target/release/freezr-daemon /usr/local/bin/

# Create config directory
sudo mkdir -p /etc/freezr

# Generate config
sudo freezr-daemon generate-config --output /etc/freezr/config.toml

# Edit config as needed
sudo nano /etc/freezr/config.toml

# Run as daemon
sudo freezr-daemon --config /etc/freezr/config.toml watch
```

### Custom Configuration
```bash
# Create custom config
cat > custom.toml << EOF
[kesl]
cpu_threshold = 40.0
memory_threshold_mb = 800
max_violations = 5

[node]
enabled = false

[monitoring]
check_interval_secs = 5
EOF

# Run with custom config
./target/release/freezr-daemon --config custom.toml watch
```

## Troubleshooting

### KESL Not Found
```
WARN KESL process not found
```
**Solution**: KESL service is not running. Start it with `sudo systemctl start kesl`.

### Permission Denied
```
ERROR Failed to restart service: Permission denied
```
**Solution**: Run with sudo for service management: `sudo freezr-daemon watch`.

### Config Not Found
```
WARN Configuration file not found: "/etc/freezr/config.toml", using defaults
```
**Solution**: Generate config file or specify path: `--config path/to/config.toml`.

### High CPU Usage from Daemon
**Solution**: Increase `check_interval_secs` in config (default 3s).

## Comparison with Bash Script

| Feature | kesl_auto_limit.sh | freezr-daemon |
|---------|-------------------|---------------|
| Language | Bash | Rust |
| Performance | ~5-10% CPU | <0.5% CPU |
| Memory | ~20MB | ~3MB |
| Reliability | Moderate | High |
| Type Safety | None | Strong |
| Error Handling | Basic | Comprehensive |
| Testing | Manual | 72 unit tests |
| Logging | echo to file | tracing + rotation |
| Configuration | Hardcoded | TOML file |
| Maintainability | Low | High |

## What's Implemented

✅ **Core Features:**
- KESL process monitoring (CPU, memory)
- Node.js process monitoring and auto-kill
- Violation counter system (max 3)
- Automatic service restart with daemon-reload
- Restart protection (100s minimum interval)
- TOML configuration
- File logging with rotation
- CLI with subcommands

✅ **Code Quality:**
- 2,820 lines of Rust code
- 72 unit + integration tests
- ~85% test coverage
- Zero unsafe code
- Comprehensive error handling

## What's Planned

🚧 **Future Features:**
- Web dashboard
- Desktop notifications
- ML-based predictions
- Thermal monitoring
- Process freezing (SIGSTOP/SIGCONT)
- Custom rules engine
- Plugin system
- Multi-service support
