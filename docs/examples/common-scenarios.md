# FreezR Common Usage Scenarios

## Scenario 1: Monitoring KESL Service

### Problem
KESL antivirus occasionally consumes excessive CPU (>30%) or memory (>600MB), causing system slowdowns.

### Solution

**Configuration (`kesl-only.toml`):**
```toml
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
enabled = false  # Disable Node.js monitoring

[monitoring]
check_interval_secs = 3
min_restart_interval_secs = 100
```

**Run:**
```bash
./target/release/freezr-daemon --config kesl-only.toml watch
```

**Expected Behavior:**
1. Checks KESL every 3 seconds
2. Tracks CPU/memory violations
3. Restarts service after 3 consecutive violations
4. Won't restart more than once per 100 seconds

**Log Output:**
```
INFO KESL process: PID 12345, CPU 35.2%, Memory 450MB
WARN CPU violation #1: 35.2% > 30.0%
INFO KESL process: PID 12345, CPU 38.5%, Memory 470MB
WARN CPU violation #2: 38.5% > 30.0%
INFO KESL process: PID 12345, CPU 40.1%, Memory 490MB
WARN CPU violation #3: 40.1% > 30.0%
ERROR Max violations reached (CPU: 3, Memory: 0), restarting service
INFO Restarting KESL service with daemon-reload
INFO KESL service successfully restarted, violations reset
```

---

## Scenario 2: Monitoring Node.js Processes

### Problem
Node.js processes occasionally hang with 100% CPU usage, requiring manual intervention.

### Solution

**Configuration (`node-only.toml`):**
```toml
[kesl]
enabled = false  # Disable KESL monitoring

[node]
cpu_threshold = 80.0
enabled = true
auto_kill = true
confirm_kill = false

[monitoring]
check_interval_secs = 5  # Check every 5 seconds
```

**Run:**
```bash
./target/release/freezr-daemon --config node-only.toml watch
```

**Expected Behavior:**
1. Scans all Node.js processes every 5 seconds
2. Kills any process with CPU >80% immediately
3. No confirmation required (auto_kill = true)

**Log Output:**
```
INFO Found 3 Node.js processes
INFO Node.js process: PID 67890, CPU 15.3%, Command: node server.js
INFO Node.js process: PID 67891, CPU 98.7%, Command: node worker.js
WARN High-CPU Node.js process: PID 67891, CPU 98.7%, Command: node worker.js
INFO Auto-killing Node.js process PID 67891 (CPU 98.7%)
INFO Successfully killed Node.js process 67891
```

---

## Scenario 3: Combined KESL + Node Monitoring

### Problem
Need to monitor both KESL and Node.js processes with different thresholds.

### Solution

**Configuration (`combined.toml`):**
```toml
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 85.0  # Higher threshold for Node
enabled = true
auto_kill = true
confirm_kill = false

[logging]
log_dir = "/var/log/freezr"
kesl_log = "kesl-monitor.log"
node_log = "node-monitor.log"
actions_log = "actions.log"

[monitoring]
check_interval_secs = 3
min_restart_interval_secs = 100
```

**Run as systemd service:**
```bash
sudo cp combined.toml /etc/freezr/config.toml
sudo systemctl restart freezr
```

---

## Scenario 4: Conservative Monitoring (Testing)

### Problem
Want to test FreezR without aggressive actions.

### Solution

**Configuration (`conservative.toml`):**
```toml
[kesl]
cpu_threshold = 50.0        # Higher threshold
memory_threshold_mb = 1024  # Higher threshold
max_violations = 5          # More violations allowed
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 95.0        # Very high threshold
enabled = true
auto_kill = false           # Don't kill, just log
confirm_kill = true

[monitoring]
check_interval_secs = 10    # Less frequent checks
min_restart_interval_secs = 300  # Longer restart interval
```

**Run:**
```bash
./target/release/freezr-daemon --config conservative.toml monitor
```

**Expected Behavior:**
- Only logs violations, doesn't take action
- Good for understanding normal system behavior
- Can adjust thresholds based on observations

---

## Scenario 5: Aggressive Protection (Production Server)

### Problem
Production server must never freeze, prefer killing processes over system hang.

### Solution

**Configuration (`aggressive.toml`):**
```toml
[kesl]
cpu_threshold = 25.0        # Lower threshold
memory_threshold_mb = 512   # Stricter memory limit
max_violations = 2          # Restart sooner
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 70.0        # Lower threshold
enabled = true
auto_kill = true
confirm_kill = false

[monitoring]
check_interval_secs = 2     # More frequent checks
min_restart_interval_secs = 60  # Allow more frequent restarts
```

**Run as systemd service with resource limits:**
```ini
[Service]
ExecStart=/usr/local/bin/freezr-daemon --config /etc/freezr/aggressive.toml watch
CPUQuota=10%   # Allow daemon more CPU for faster response
MemoryMax=100M
```

---

## Scenario 6: Development Environment

### Problem
Development machine with Node.js servers, Docker, and other tools. Want to prevent hangs without killing important processes.

### Solution

**Configuration (`dev.toml`):**
```toml
[kesl]
enabled = false  # No KESL on dev machine

[node]
cpu_threshold = 90.0  # High threshold for npm/webpack
enabled = true
auto_kill = true
confirm_kill = false

[logging]
log_dir = "./logs"  # Local logs
max_file_size_mb = 5
rotate_count = 3

[monitoring]
check_interval_secs = 5
```

**Run in terminal:**
```bash
./target/release/freezr-daemon --config dev.toml watch
```

**Use Case:**
- Catches runaway webpack builds
- Prevents npm install hangs
- Doesn't interfere with normal development

---

## Scenario 7: Scheduled Monitoring (Cron)

### Problem
Want periodic checks without continuous daemon.

### Solution

**Add to crontab:**
```bash
# Check every 5 minutes, log results
*/5 * * * * /usr/local/bin/freezr-daemon --config /etc/freezr/config.toml monitor >> /var/log/freezr/cron.log 2>&1
```

**Configuration (`cron.toml`):**
```toml
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 1  # Single check, so 1 violation = action
enabled = true

[node]
enabled = false  # Disable for cron

[monitoring]
check_interval_secs = 1  # Not used in monitor mode
```

**Log Analysis:**
```bash
# View cron results
tail -f /var/log/freezr/cron.log

# Count violations
grep "violation" /var/log/freezr/cron.log | wc -l

# Find restart events
grep "restart" /var/log/freezr/cron.log
```

---

## Scenario 8: Multi-Instance Deployment

### Problem
Need different monitoring policies for different services.

### Solution

**KESL Instance (`/etc/freezr/kesl.toml`):**
```toml
[kesl]
enabled = true
cpu_threshold = 30.0
memory_threshold_mb = 600

[node]
enabled = false

[logging]
log_dir = "/var/log/freezr/kesl"
```

**Node Instance (`/etc/freezr/node.toml`):**
```toml
[kesl]
enabled = false

[node]
enabled = true
cpu_threshold = 80.0
auto_kill = true

[logging]
log_dir = "/var/log/freezr/node"
```

**Systemd Services:**
```bash
# Create separate service files
sudo cp /etc/systemd/system/freezr.service /etc/systemd/system/freezr-kesl.service
sudo cp /etc/systemd/system/freezr.service /etc/systemd/system/freezr-node.service

# Edit ExecStart in each:
# freezr-kesl.service: --config /etc/freezr/kesl.toml
# freezr-node.service: --config /etc/freezr/node.toml

# Start both
sudo systemctl daemon-reload
sudo systemctl enable freezr-kesl freezr-node
sudo systemctl start freezr-kesl freezr-node
```

---

## Scenario 9: Debugging and Troubleshooting

### Problem
FreezR isn't working as expected, need detailed logs.

### Solution

**Enable debug logging:**
```bash
RUST_LOG=debug ./target/release/freezr-daemon --config debug.toml watch
```

**Configuration (`debug.toml`):**
```toml
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
enabled = true

[node]
enabled = true
cpu_threshold = 80.0

[logging]
log_dir = "./debug-logs"
max_file_size_mb = 100  # Larger logs for debugging
rotate_count = 10

[monitoring]
check_interval_secs = 1  # Faster checks for debugging
```

**View detailed logs:**
```bash
# Real-time debug output
tail -f debug-logs/freezr-daemon.log.$(date +%Y-%m-%d)

# Filter for specific process
grep "PID 12345" debug-logs/*.log

# View all violations
grep -i "violation" debug-logs/*.log | sort
```

---

## Scenario 10: Dry-Run Mode (Simulation)

### Problem
Want to see what FreezR would do without actually taking actions.

### Solution

Currently, FreezR doesn't have a dry-run mode, but you can simulate it by:

**Option 1: Use conservative thresholds**
```toml
[kesl]
cpu_threshold = 99.0  # Will never trigger
memory_threshold_mb = 99999
max_violations = 999

[node]
cpu_threshold = 99.0
auto_kill = false  # Don't kill
```

**Option 2: Monitor logs without actions**
```bash
# Run monitor mode repeatedly
while true; do
  ./target/release/freezr-daemon monitor
  sleep 5
done
```

**Option 3: Analyze logs**
```bash
# Count potential violations
grep "CPU:" logs/*.log | awk '{if ($4 > 30) print $0}'
```

---

## Quick Reference

| Use Case | Config Highlights | Run Mode |
|----------|------------------|----------|
| KESL Only | `kesl.enabled=true`, `node.enabled=false` | `watch` |
| Node Only | `kesl.enabled=false`, `node.enabled=true` | `watch` |
| Conservative | High thresholds, `auto_kill=false` | `monitor` |
| Aggressive | Low thresholds, max_violations=2 | `watch` |
| Development | `node.enabled=true`, high threshold | `watch` |
| Production | Low thresholds, frequent checks | systemd |
| Testing | Single check, no actions | `monitor` |
| Debugging | `RUST_LOG=debug`, check_interval=1 | `watch` |

## Best Practices

1. **Start conservative** - Use high thresholds and test before production
2. **Monitor logs** - Review logs regularly to understand normal behavior
3. **Adjust thresholds** - Fine-tune based on actual process behavior
4. **Use systemd** - For production, always use systemd service
5. **Separate instances** - Use different configs for different monitoring needs
6. **Test restarts** - Verify service restarts don't cause issues
7. **Log rotation** - Ensure logs don't fill disk space
8. **Resource limits** - Limit FreezR's own CPU/memory usage

## Next Steps

- [Configuration Guide](../user-guide/configuration.md) - Detailed config options
- [Systemd Integration](../deployment/systemd.md) - Production deployment
- [Troubleshooting](../user-guide/getting-started.md#troubleshooting) - Common issues
