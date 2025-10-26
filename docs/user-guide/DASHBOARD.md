# FreezR Dashboard Guide

## Overview

FreezR provides two modes of operation:
- **Service Mode**: Runs monitoring and performs actions (kill, freeze, restart)
- **Dashboard Mode**: Read-only viewer that displays live statistics

This architecture allows you to run the service in the background (via systemd or manually) while viewing statistics in a separate terminal window.

## Architecture

```
┌─────────────────────────────────────────┐
│  Service (systemd or manual)            │
│  process-monitor --stats                │
│                                          │
│  ┌────────────────────────────────────┐ │
│  │  Monitor Loop                      │ │
│  │  - Check KESL, Node, Firefox, etc. │ │
│  │  - Take actions (kill, freeze)     │ │
│  │  - Export stats to JSON file ──────┼─┐
│  └────────────────────────────────────┘ │ │
└─────────────────────────────────────────┘ │
                                             │
                        /tmp/freezr-stats.json
                                             │
┌────────────────────────────────────────────┼┐
│  Dashboard (on-demand viewer)              ││
│  process-monitor dashboard                 ││
│                                             ││
│  ┌────────────────────────────────────┐   ││
│  │  Display Loop                      │   ││
│  │  - Read JSON file every N secs ◄───────┘│
│  │  - Display full dashboard UI       │    │
│  │  - NO monitoring, NO actions       │    │
│  └────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

## Quick Start

### Using Bash Aliases

The easiest way to use FreezR is via bash aliases:

```bash
# Apply aliases (run once)
source ~/.bashrc

# Start service (monitoring + actions)
keslwatchR

# In another terminal: Start dashboard (view only)
keslwatchRmon
```

### Manual Commands

**Service Mode (monitoring + export stats):**
```bash
cd /home/ryazanov/.myBashScripts/freezr
./target/release/process-monitor --config freezr.toml --stats --report-interval 60
```

**Dashboard Mode (view only):**
```bash
cd /home/ryazanov/.myBashScripts/freezr
./target/release/process-monitor dashboard --interval 3
```

## Service Mode

### Running as Systemd Service (Recommended)

**Install service:**
```bash
./target/release/process-monitor install-service
```

**Manage service:**
```bash
# Start service
sudo systemctl start freezr

# Stop service
sudo systemctl stop freezr

# Restart service
sudo systemctl restart freezr

# Check status
sudo systemctl status freezr

# View logs
sudo journalctl -u freezr -f
```

**Check service status from CLI:**
```bash
./target/release/process-monitor service-status
```

**Uninstall service:**
```bash
./target/release/process-monitor uninstall-service
```

### Running Manually

**Background mode:**
```bash
# Start in background
./target/release/process-monitor --config freezr.toml --stats --report-interval 60 &

# Check if running
ps aux | grep process-monitor

# Stop
pkill -f "process-monitor --config"
```

**Foreground mode:**
```bash
# Shows live dashboard with statistics
./target/release/process-monitor --config freezr.toml --stats --report-interval 60
```

## Dashboard Mode

### Basic Usage

**Default interval (3 seconds):**
```bash
./target/release/process-monitor dashboard
```

**Custom interval:**
```bash
# Update every 2 seconds
./target/release/process-monitor dashboard --interval 2

# Update every 5 seconds
./target/release/process-monitor dashboard --interval 5
```

**Using alias:**
```bash
# Default 3-second interval
keslwatchRmon
```

### Dashboard Features

The dashboard displays:

1. **Runtime Information**
   - Total uptime
   - Total monitoring checks performed
   - Check interval

2. **KESL Process Status**
   - Current PID
   - CPU usage (current vs threshold)
   - Memory usage (current vs threshold)

3. **Violations Summary**
   - Total CPU violations
   - Total memory violations
   - Current session violations
   - Violation rate percentage

4. **Actions Summary**
   - KESL restarts count
   - Node.js kills count
   - Snap actions count
   - Firefox freeze/kill stats
   - Brave freeze/kill stats
   - Telegram freeze/kill stats
   - Memory pressure status

5. **System Health**
   - Load average (1, 5, 15 min)
   - Memory usage percentage

6. **Log Statistics**
   - Active log files count and size
   - Archived log files count and size
   - Retention policy

### Dashboard Output Example

```
╔═══════════════════════════════════════════════════════════╗
║       FreezR Live Dashboard (Read-Only Mode)             ║
╚═══════════════════════════════════════════════════════════╝

📡 Reading stats from: /tmp/freezr-stats.json
🔄 Update interval: 3 seconds
Press Ctrl+C to exit

╔═══════════════════════════════════════════════════════════╗
║          FreezR Process Monitor - Live Dashboard         ║
╚═══════════════════════════════════════════════════════════╝

📈 Runtime: 0h 15m 42s
📊 Total checks: 314 (every 3s)

╔═══════════════════════════════════════════════════════════╗
║                    KESL Process Status                    ║
╚═══════════════════════════════════════════════════════════╝
   PID: 495246 (current)
   CPU: 20.0% (threshold: 30.0%)
   Memory: 451MB (threshold: 600MB)

╔═══════════════════════════════════════════════════════════╗
║                   Violations Summary                      ║
╚═══════════════════════════════════════════════════════════╝
   Total:
      CPU violations: 5
      Memory violations: 0
   Current session:
      CPU: 0 (need 3 for restart)
      Memory: 0 (need 3 for restart)
   Violation rate: 1.59%

╔═══════════════════════════════════════════════════════════╗
║                    Actions Summary                        ║
╚═══════════════════════════════════════════════════════════╝
   🔄 KESL restarts: 0
   🔪 Node.js kills: 2
   ⚡ Snap actions: 0 (nice)
   🦊 Firefox: Freeze@80.0%, Kill@95.0%
   🦁 Brave: Freeze@80.0%, Kill@95.0%
   ✈️  Telegram: Freeze@80.0%, Kill@95.0%
   ⚪ Memory Pressure: OK (some: 0.0%, full: 0.0%, w:0/c:0)

╔═══════════════════════════════════════════════════════════╗
║                     System Health                         ║
╚═══════════════════════════════════════════════════════════╝
   Load: 2.43, Memory: 60.8% used

╔═══════════════════════════════════════════════════════════╗
║                    Log Statistics                         ║
╚═══════════════════════════════════════════════════════════╝
   📄 Active logs: 4 files (1.9M)
   🗜️  Archive logs: 0 files (4.0K)
   📊 Retention: 7 days active, 30 days archived

Press Ctrl+C to stop monitoring
Next refresh in 3s...
```

## Usage Scenarios

### Scenario 1: Development

**Terminal 1 - Service:**
```bash
# Run service in foreground to see actions
./target/release/process-monitor --config freezr.toml --stats --report-interval 60
```

**Terminal 2 - Dashboard:**
```bash
# View live stats
keslwatchRmon
```

### Scenario 2: Production

**Install and enable systemd service:**
```bash
# Install service
./target/release/process-monitor install-service

# Start and enable
sudo systemctl enable --now freezr
```

**View dashboard when needed:**
```bash
# Open dashboard in any terminal
keslwatchRmon
```

**View service logs:**
```bash
# Follow service logs
sudo journalctl -u freezr -f

# View recent logs
sudo journalctl -u freezr -n 100

# View logs since boot
sudo journalctl -u freezr -b
```

### Scenario 3: Testing Configuration

**Test without systemd:**
```bash
# Terminal 1: Run with custom config
./target/release/process-monitor --config test.toml --stats --report-interval 10

# Terminal 2: Watch results
keslwatchRmon
```

## Stats Export

### JSON File Location

Stats are exported to: `/tmp/freezr-stats.json`

### Stats File Structure

```json
{
  "timestamp": 1761488426,
  "runtime_secs": 942,
  "total_checks": 314,
  "kesl": {
    "pid": 495246,
    "cpu_percent": 20.0,
    "memory_mb": 451,
    "cpu_threshold": 30.0,
    "memory_threshold_mb": 600,
    "total_cpu_violations": 5,
    "total_memory_violations": 0,
    "current_cpu_violations": 0,
    "current_memory_violations": 0,
    "max_violations": 3,
    "violation_rate": 1.59,
    "total_restarts": 0
  },
  "node": {
    "enabled": true,
    "cpu_threshold": 80.0,
    "auto_kill": true,
    "total_kills": 2
  },
  // ... more stats
}
```

### Reading Stats Programmatically

**Bash:**
```bash
# Read current CPU usage
cat /tmp/freezr-stats.json | jq '.kesl.cpu_percent'

# Read total violations
cat /tmp/freezr-stats.json | jq '.kesl.total_cpu_violations'

# Read memory pressure status
cat /tmp/freezr-stats.json | jq '.memory_pressure.status'
```

**Python:**
```python
import json

with open('/tmp/freezr-stats.json') as f:
    stats = json.load(f)

print(f"KESL CPU: {stats['kesl']['cpu_percent']}%")
print(f"Total checks: {stats['total_checks']}")
print(f"Memory pressure: {stats['memory_pressure']['status']}")
```

## Troubleshooting

### Dashboard shows "Stats file not found"

**Problem:** Dashboard cannot find `/tmp/freezr-stats.json`

**Solutions:**
1. Check if service is running:
   ```bash
   systemctl status freezr
   # or
   ps aux | grep process-monitor
   ```

2. Verify stats file exists:
   ```bash
   ls -lh /tmp/freezr-stats.json
   ```

3. Start service:
   ```bash
   sudo systemctl start freezr
   # or
   keslwatchR &
   ```

### Dashboard shows old data

**Problem:** Stats file not being updated

**Solutions:**
1. Check service is running in --stats mode:
   ```bash
   ps aux | grep "process-monitor --config" | grep stats
   ```

2. Restart service:
   ```bash
   sudo systemctl restart freezr
   ```

3. Check file modification time:
   ```bash
   stat /tmp/freezr-stats.json
   ```

### Multiple service instances

**Problem:** Multiple process-monitor instances running

**Solutions:**
1. Check all instances:
   ```bash
   ps aux | grep process-monitor | grep -v grep
   ```

2. Stop all instances:
   ```bash
   pkill -f "process-monitor --config"
   sudo systemctl stop freezr
   ```

3. Start only one service:
   ```bash
   sudo systemctl start freezr
   ```

## Best Practices

### Production Deployment

1. **Use systemd service** for automatic startup and restart
2. **Monitor service logs** with journalctl
3. **Use dashboard** for visual monitoring when needed
4. **Set appropriate intervals** (60s for service, 3s for dashboard)

### Development Workflow

1. **Run service in foreground** to see actions
2. **Use dashboard in separate terminal** for stats
3. **Test configuration changes** before applying to systemd
4. **Check logs** after configuration changes

### Resource Management

1. **Service mode**: Runs continuously, exports stats every check
2. **Dashboard mode**: Lightweight, read-only, no monitoring overhead
3. **Multiple dashboards**: Safe to run multiple viewers
4. **Stats file**: Auto-updated, 2-3 KB size, minimal disk I/O

## Command Reference

### Service Management

```bash
# Install systemd service
./target/release/process-monitor install-service

# Uninstall systemd service
./target/release/process-monitor uninstall-service

# Check service status
./target/release/process-monitor service-status

# Start service manually
./target/release/process-monitor --config freezr.toml --stats --report-interval 60
```

### Dashboard

```bash
# Start dashboard with default interval (3s)
./target/release/process-monitor dashboard

# Start dashboard with custom interval
./target/release/process-monitor dashboard --interval 5

# Using alias
keslwatchRmon
```

### Systemd

```bash
# Start service
sudo systemctl start freezr

# Stop service
sudo systemctl stop freezr

# Restart service
sudo systemctl restart freezr

# Enable auto-start
sudo systemctl enable freezr

# Disable auto-start
sudo systemctl disable freezr

# View status
sudo systemctl status freezr

# View logs
sudo journalctl -u freezr -f
```

## See Also

- [Getting Started Guide](getting-started.md)
- [Systemd Service Documentation](../technical/SYSTEMD_SERVICE.md)
- [Memory Pressure Monitoring](../technical/MEMORY_PRESSURE.md)
- [Configuration Reference](../config/README.md)
