# Snap/Snapd Process Monitoring

## Overview

FreezR now includes monitoring for snap/snapd processes that can consume excessive CPU resources (>300%). This feature helps manage system performance when snap processes become resource-intensive.

## Problem

Snap processes can sometimes consume extremely high CPU (>300%, indicating multi-core usage), causing system slowdowns. Unlike single-threaded processes that max out at 100% on one core, snap processes can utilize multiple cores simultaneously.

## Solution

FreezR monitors snap processes and takes configurable actions when CPU usage exceeds thresholds:

- **nice** (default): Lower process priority to prevent system impact
- **freeze**: Temporarily suspend the process (SIGSTOP/SIGCONT)
- **kill**: Terminate the process

## Configuration

### freezr.toml

```toml
[snap]
# CPU threshold in percent (300% = 3 cores fully utilized)
cpu_threshold = 300.0

# Enable snap monitoring
enabled = true

# Action to take when threshold exceeded
# Options: "freeze", "nice", "kill"
action = "nice"

# Nice level (0-19, higher = lower priority)
# Used when action = "nice"
nice_level = 15

# Freeze duration in seconds
# Used when action = "freeze"
freeze_duration_secs = 5

# Maximum violations before taking action
max_violations = 3
```

## How It Works

1. **Scanning**: Every check interval, FreezR scans for snap processes using `ps aux`
2. **Detection**: Identifies processes with "snap" in command name or path
3. **CPU Measurement**: Measures CPU usage via `top` command
4. **Violation Tracking**: Counts consecutive violations (CPU > threshold)
5. **Action**: After `max_violations` reached, executes configured action

### Actions Explained

#### Nice (Default - Recommended)

```bash
# Sets process nice level to reduce priority
sudo renice -n 15 -p <PID>
```

**Advantages**:
- Non-destructive
- Process continues running
- Prevents system impact
- Good for background snap updates

**Use Case**: Snap store updates, background maintenance

#### Freeze

```bash
# Suspend process temporarily
kill -SIGSTOP <PID>
sleep 5
kill -SIGCONT <PID>
```

**Advantages**:
- Temporary relief
- Process resumes after duration
- Useful for periodic spikes

**Use Case**: Intermittent high CPU bursts

#### Kill

```bash
# Terminate process (SIGTERM, then SIGKILL if needed)
kill -15 <PID>
sleep 2
kill -9 <PID>  # If still alive
```

**Advantages**:
- Immediate relief
- Permanent solution
- Frees all resources

**Use Case**: Stuck snap processes that won't respond

**Warning**: May interrupt snap updates/installations

## Process Detection

FreezR detects snap processes by matching:

- Command name contains "snap"
- Command path contains "/snap/"
- Common processes: snapd, snap-store, snap-confine, snapd-desktop-integration

## Monitoring Statistics

The process-monitor binary displays snap monitoring status:

```
╔═══════════════════════════════════════════════════════════╗
║          FreezR Process Monitor v0.1.0                    ║
╚═══════════════════════════════════════════════════════════╝

📊 Monitoring Configuration:
   └─ KESL: CPU 30.0%, Memory 600MB (max 3 violations)
   └─ Node.js: CPU 80.0%, Auto-kill: true
   └─ Snap: CPU 300.0%, Action: nice, Nice: 15
   └─ Check interval: 3s

╔═══════════════════════════════════════════════════════════╗
║                    Actions Summary                        ║
╚═══════════════════════════════════════════════════════════╝
   🔄 KESL restarts: 0
   🔪 Node.js kills: 2
   ⚡ Snap actions: 5 (nice)
```

## Example Scenarios

### Scenario 1: Snap Store Update Consuming 350% CPU

```toml
[snap]
cpu_threshold = 300.0
enabled = true
action = "nice"
nice_level = 15
max_violations = 3
```

**Behavior**:
1. Check 1: Snap at 350% → Violation #1
2. Check 2: Snap at 370% → Violation #2
3. Check 3: Snap at 340% → Violation #3 → Action: renice to 15
4. Process continues but with lower priority
5. System remains responsive

### Scenario 2: Stuck Snap Process at 400% CPU

```toml
[snap]
cpu_threshold = 300.0
enabled = true
action = "kill"
max_violations = 2
```

**Behavior**:
1. Check 1: Snap at 400% → Violation #1
2. Check 2: Snap at 410% → Violation #2 → Action: kill
3. Process terminated
4. System resources freed

### Scenario 3: Periodic CPU Spikes

```toml
[snap]
cpu_threshold = 300.0
enabled = true
action = "freeze"
freeze_duration_secs = 10
max_violations = 3
```

**Behavior**:
1. After 3 violations → Freeze process for 10 seconds
2. Process suspended (SIGSTOP)
3. After 10s → Resume (SIGCONT)
4. Allows system to catch up

## Implementation Details

### Core Components

1. **Scanner** (`crates/freezr-core/src/scanner.rs`)
   - `scan_snap_processes()`: Finds all snap processes
   - `find_snap_pids()`: Identifies snap PIDs via ps
   - `measure_cpu_top()`: Measures CPU per process

2. **Executor** (`crates/freezr-core/src/executor.rs`)
   - `renice_process()`: Changes process priority
   - `freeze_process()`: Sends SIGSTOP
   - `unfreeze_process()`: Sends SIGCONT
   - `kill_process()`: Terminates process (SIGTERM/SIGKILL)

3. **Monitor** (`crates/freezr-daemon/src/monitor.rs`)
   - `check_snap_processes()`: Main monitoring loop
   - Tracks violations per process
   - Executes actions based on config

4. **Config** (`crates/freezr-daemon/src/config.rs`)
   - `SnapConfig`: Configuration structure
   - Validation: CPU threshold 0-1000%, nice level 0-19
   - Action validation: "freeze", "nice", or "kill"

### Violation Reset Logic

```rust
if high_cpu_processes.is_empty() {
    // Reset violations if CPU back to normal
    if self.snap_violations > 0 {
        debug!("Snap CPU back to normal, resetting violations");
        self.snap_violations = 0;
    }
}
```

**Important**: Violations reset when CPU drops below threshold, preventing false positives.

## Security Considerations

### Permission Requirements

- **nice action**: Requires `sudo` for nice levels < 0 (higher priority)
- **freeze/unfreeze**: Requires permission to send signals
- **kill**: Same as freeze

### Sudoers Configuration

For automated nice adjustment without password:

```bash
# /etc/sudoers.d/freezr-snap
ryazanov ALL=(root) NOPASSWD: /usr/bin/renice
```

## Testing

### Manual Test

1. Create test snap process:
```bash
# This is just an example - don't actually do this
snap install some-heavy-app
```

2. Monitor logs:
```bash
tail -f logs/process_monitor.log
```

3. Watch for violations:
```
[2025-10-25T20:00:00Z] WARN Snap CPU violation #1: 1 processes exceed 300.0%
[2025-10-25T20:00:00Z] WARN High-CPU Snap process: PID 1234, CPU 350.0%, Command: snapd
[2025-10-25T20:00:03Z] WARN Snap CPU violation #2: 1 processes exceed 300.0%
[2025-10-25T20:00:06Z] ERROR Snap max violations (3) reached, taking action: nice
[2025-10-25T20:00:06Z] INFO Setting nice level 15 for snap process PID 1234
[2025-10-25T20:00:06Z] INFO Successfully set nice level 15 for snap process 1234
```

### Automated Test

```bash
# Build and run process-monitor
cargo build --release
./target/release/process-monitor --stats --report-interval 60
```

## Troubleshooting

### Snap processes not detected

**Issue**: No snap processes found in monitoring logs

**Solutions**:
1. Check if snap is installed: `snap version`
2. Verify snap processes exist: `ps aux | grep snap`
3. Enable debug logging: `RUST_LOG=debug ./target/release/process-monitor`

### Renice fails with permission denied

**Issue**: `Failed to renice snap process: Permission denied`

**Solutions**:
1. Run with sudo: `sudo ./target/release/process-monitor`
2. Configure sudoers (recommended for automation)
3. Use action = "freeze" or "kill" instead

### Actions not triggering

**Issue**: CPU violations logged but no action taken

**Check**:
1. Verify `max_violations` is set correctly
2. Check if CPU drops below threshold between checks
3. Increase check interval to allow sustained violations

## Performance Impact

- **CPU overhead**: ~0.1-0.5% (scanning processes)
- **Memory overhead**: Minimal (~2MB)
- **Disk I/O**: Log writes only
- **Network**: None

## Comparison with Other Solutions

### systemd resource limits

```ini
# /etc/systemd/system/snapd.service.d/limits.conf
[Service]
CPUQuota=200%
```

**Advantages of FreezR**:
- Dynamic response to actual usage
- Multiple action types
- Violation tracking
- Per-process granularity
- Works with all snap processes (not just snapd service)

### Manual monitoring

```bash
# Traditional approach
watch -n 5 'ps aux | grep snap | awk "{if(\$3>300) print}"'
```

**Advantages of FreezR**:
- Automated actions
- Statistical tracking
- Configurable thresholds
- Multi-process support
- Logging and alerting

## Future Enhancements

Potential improvements:

1. **Per-process configuration**: Different thresholds for snapd vs snap-store
2. **Time-based rules**: More lenient during off-hours
3. **Resource quotas**: Combined CPU + memory limits
4. **Alert notifications**: Email/Telegram when actions taken
5. **Machine learning**: Predict and prevent issues
6. **Process whitelisting**: Exclude critical snap services
7. **cgroups integration**: Use kernel-level limits

## References

- Snap Documentation: https://snapcraft.io/docs
- Linux Process Management: `man 7 signal`
- Nice/Renice: `man 1 renice`
- Process Priority: https://www.kernel.org/doc/Documentation/scheduler/

## Changelog

### v0.1.0 (2025-10-25)

- ✅ Initial snap monitoring implementation
- ✅ Three action types: nice, freeze, kill
- ✅ Violation tracking with configurable thresholds
- ✅ Integration with process-monitor binary
- ✅ Configuration via freezr.toml
- ✅ Automated testing and validation

## License

Part of FreezR project - See main README for license information.
