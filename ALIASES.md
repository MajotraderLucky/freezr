# FreezR Shell Aliases

Convenient shell aliases for FreezR (Rust version).

## Available Aliases

### `keslwatchR`
**Continuous monitoring with auto-actions**

```bash
keslwatchR
```

**What it does:**
- Monitors KESL process every 3 seconds
- Tracks CPU (30%) and Memory (600MB) violations
- Auto-restarts KESL service after 3 violations
- Monitors Node.js processes (CPU >80%)
- Auto-kills hung Node.js processes
- Logs everything to daily log files

**Use when:**
- You want active protection against runaway processes
- Production monitoring
- Long-running daemon mode

**Stop with:** `Ctrl+C`

---

### `keslmonR`
**Single check with status report**

```bash
keslmonR
```

**What it does:**
- Performs one monitoring check
- Shows current statistics
- Displays violations count
- Reports KESL process status
- Exits after check

**Use when:**
- Quick status check
- Testing configuration
- Manual inspection
- Cron jobs

---

### `keslrestartR`
**Force restart KESL service**

```bash
keslrestartR
```

**What it does:**
- Immediately restarts KESL service
- Runs systemctl daemon-reload
- Applies resource limits
- Requires sudo (password prompt)

**Use when:**
- Manual service restart needed
- KESL is stuck
- Testing restart functionality
- Emergency recovery

---

### `kesllogR`
**View real-time logs**

```bash
kesllogR
```

**What it does:**
- Opens today's log file
- Follows logs in real-time (tail -f)
- Shows all FreezR daemon activity

**Use when:**
- Monitoring daemon activity
- Debugging issues
- Checking violation history
- Viewing restart events

**Stop with:** `Ctrl+C`

---

## Comparison with Bash Version

| Feature | kesl_auto_limit.sh | FreezR (Rust) |
|---------|-------------------|---------------|
| Alias | `keslwatch` | `keslwatchR` |
| CPU Usage | 5-10% | <0.5% |
| Memory | ~20MB | ~3MB |
| Speed | 1x | 10-20x faster |
| Type Safety | None | Strong |
| Tests | Manual | 72 automated |

## Usage Examples

### Example 1: Start Monitoring
```bash
# Start FreezR daemon
keslwatchR

# Expected output:
# INFO FreezR Daemon v0.1.0 starting...
# INFO Configuration loaded and validated successfully
# INFO Starting continuous monitoring loop...
# INFO Check interval: 3s, Max violations: 3
# WARN Node.js monitoring enabled: CPU threshold 80.0%, auto-kill: true
```

### Example 2: Quick Status Check
```bash
# Check current status
keslmonR

# Expected output:
# INFO === Monitoring Status ===
# INFO Total checks: 1
# INFO CPU violations: 0 (current session: 0)
# INFO Memory violations: 0 (current session: 0)
# INFO Total restarts: 0
# INFO Total kills: 0
```

### Example 3: View Logs
```bash
# Start watching logs
kesllogR

# In another terminal, start monitoring
keslwatchR

# You'll see real-time log updates:
# 2025-10-25T18:05:11.669629Z  INFO freezr_daemon: KESL process: PID 122326, CPU 0.0%, Memory 2MB
# 2025-10-25T18:05:11.689820Z  INFO freezr_daemon: Stats: checks=1, violations=0/0, restarts=0, kills=0
```

### Example 4: Force Restart KESL
```bash
# Restart KESL service
keslrestartR

# Enter sudo password when prompted
# Expected output:
# INFO Forcing KESL service restart...
# INFO KESL service restarted successfully
```

## Workflow Examples

### Daily Monitoring
```bash
# Morning: Start monitoring
keslwatchR

# Leave running in background or tmux session
# Check status anytime with:
keslmonR

# View logs if needed:
kesllogR
```

### Troubleshooting Workflow
```bash
# Step 1: Check current status
keslmonR

# Step 2: If KESL is problematic, force restart
keslrestartR

# Step 3: Start monitoring to prevent future issues
keslwatchR

# Step 4: In separate terminal, watch logs
kesllogR
```

### Testing Configuration
```bash
# Check status without running daemon
keslmonR

# Review output, adjust config if needed
nano /home/ryazanov/.myBashScripts/freezr/config/examples/config.toml

# Test with single check again
keslmonR

# If satisfied, start full monitoring
keslwatchR
```

## Configuration

Aliases use configuration from:
```
/home/ryazanov/.myBashScripts/freezr/config/examples/config.toml
```

**To customize:**
1. Copy example config:
   ```bash
   cp config/examples/config.toml config/my-config.toml
   ```

2. Edit settings:
   ```bash
   nano config/my-config.toml
   ```

3. Update alias in `~/.bashrc`:
   ```bash
   alias keslwatchR='cd /home/ryazanov/.myBashScripts/freezr && ./target/release/freezr-daemon --config config/my-config.toml watch'
   ```

4. Reload bashrc:
   ```bash
   source ~/.bashrc
   ```

## Tips & Tricks

### Run in Background
```bash
# Start in background with nohup
nohup keslwatchR > /dev/null 2>&1 &

# Or use screen/tmux
screen -S freezr
keslwatchR
# Detach with Ctrl+A, D
```

### Cron Job for Periodic Checks
```bash
# Add to crontab
*/5 * * * * /home/ryazanov/.bashrc && keslmonR >> /var/log/freezr-cron.log 2>&1
```

### Combine with Original Bash Script
```bash
# Run both for comparison
keslwatch      # Original bash version
keslwatchR     # New Rust version

# Compare CPU usage
ps aux | grep -E "kesl_auto|freezr-daemon"
```

### Quick Log Analysis
```bash
# Search logs for violations
kesllogR | grep "violation"

# Count restarts today
grep "restart" logs/freezr-daemon.log.$(date +%Y-%m-%d) | wc -l

# Find killed Node processes
grep "killed" logs/freezr-daemon.log.* | tail -20
```

## Troubleshooting Aliases

### Alias Not Found
```bash
# Reload bashrc
source ~/.bashrc

# Or restart terminal
```

### Permission Denied
```bash
# For keslrestartR, ensure sudo access configured
# Or run with sudo explicitly:
cd /home/ryazanov/.myBashScripts/freezr
sudo ./target/release/freezr-daemon force-restart
```

### Binary Not Found
```bash
# Rebuild FreezR
cd /home/ryazanov/.myBashScripts/freezr
cargo build --release

# Verify binary exists
ls -lh target/release/freezr-daemon
```

### Config Not Found
```bash
# Generate default config
cd /home/ryazanov/.myBashScripts/freezr
./target/release/freezr-daemon generate-config --output config/examples/config.toml
```

## Alias Definition

These aliases are defined in `~/.bashrc`:

```bash
# FreezR (Rust version of kesl_auto_limit)
alias keslwatchR='cd /home/ryazanov/.myBashScripts/freezr && ./target/release/freezr-daemon --config config/examples/config.toml watch'
alias keslmonR='cd /home/ryazanov/.myBashScripts/freezr && ./target/release/freezr-daemon --config config/examples/config.toml monitor'
alias keslrestartR='cd /home/ryazanov/.myBashScripts/freezr && sudo ./target/release/freezr-daemon --config config/examples/config.toml force-restart'
alias kesllogR='tail -f /home/ryazanov/.myBashScripts/freezr/logs/freezr-daemon.log.$(date +%Y-%m-%d)'
```

## See Also

- [Getting Started Guide](docs/user-guide/getting-started.md)
- [Usage Examples](docs/examples/common-scenarios.md)
- [Systemd Service](docs/deployment/systemd.md)
- [Configuration Guide](docs/user-guide/USAGE.md#configuration)
