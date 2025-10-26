# Memory Pressure Monitoring (PSI)

FreezR implements **PSI (Pressure Stall Information)** monitoring to prevent OOM (Out of Memory) situations **before** they occur. This proactive approach allows the system to take action early, before memory exhaustion causes system instability.

## Overview

Memory pressure monitoring reads from `/proc/pressure/memory` (Linux kernel PSI interface) and tracks two key metrics:

- **`some`**: Percentage of time at least one process is waiting for memory
- **`full`**: Percentage of time ALL processes are waiting for memory (critical!)

These metrics are measured over 10, 60, and 300 second windows. FreezR primarily uses the **avg10** (10-second average) for responsive action.

## Configuration

Memory pressure monitoring is configured in `freezr.toml`:

```toml
[memory_pressure]
# Enable PSI monitoring
enabled = true

# Thresholds for 'some' metric (avg10)
# "some" = at least one process waiting for memory
some_threshold_warning = 10.0    # Warning: 10% of time processes wait
some_threshold_critical = 30.0   # Critical: 30% of time - proactive actions

# Thresholds for 'full' metric (avg10)
# "full" = ALL processes waiting for memory (very bad!)
full_threshold_warning = 5.0     # Warning: 5% of time all blocked
full_threshold_critical = 15.0   # Critical: 15% of time - aggressive actions

# Actions at different levels
# Options: "log", "nice", "freeze", "kill"
action_warning = "log"           # Just log warnings
action_critical = "freeze"       # Freeze non-critical processes

# Check interval (seconds)
check_interval_secs = 5          # Check every 5 seconds
```

## Action Levels

FreezR supports four action levels when memory pressure is detected:

### 1. **log** (Passive)
- Only logs the memory pressure event
- No process manipulation
- Good for monitoring without intervention

### 2. **nice** (Gentle)
- Lowers priority of non-critical processes to nice level 15
- Processes continue running but get less CPU time
- Targets: Firefox, Brave, Telegram
- Good for light memory pressure

### 3. **freeze** (Moderate)
- Freezes non-critical processes for 5 seconds (SIGSTOP)
- Gives memory-critical processes time to allocate
- Automatically unfreezes after timeout (SIGCONT)
- Targets: Firefox, Brave, Telegram
- **Recommended for critical level**

### 4. **kill** (Aggressive)
- Terminates non-critical processes (SIGTERM → SIGKILL)
- Last resort to prevent OOM killer
- Targets: Firefox, Brave, Telegram
- **Use with caution**

## How It Works

### Detection Flow

```
Check /proc/pressure/memory every 5 seconds
    ↓
Parse 'some' and 'full' avg10 metrics
    ↓
Compare against thresholds
    ↓
If CRITICAL (some ≥ 30% OR full ≥ 15%)
    → Execute critical action (freeze)
    → Increment critical counter
    ↓
Else if WARNING (some ≥ 10% OR full ≥ 5%)
    → Execute warning action (log)
    → Increment warning counter
    ↓
Else (no pressure)
    → Reset counters
```

### Non-Critical Processes

FreezR considers these processes "non-critical" and safe to freeze/nice/kill during memory pressure:

- **Firefox** - Web browser
- **Brave** - Web browser
- **Telegram** - Messenger

These processes are chosen because:
1. They can be resource-heavy
2. They're not essential for system operation
3. Users can easily restart them
4. They often contribute to memory pressure

**Critical processes never touched:**
- KESL (Kaspersky)
- System services (systemd, sshd, etc.)
- Desktop environment (Xorg, KWin, plasmashell)
- Node.js processes (may be running important services)

## Proactive vs Reactive

### Traditional Approach (Reactive)
```
Memory fills up → OOM killer triggers → Random process killed → System recovers
```
**Problem**: OOM killer might kill critical processes, causing data loss.

### FreezR Approach (Proactive)
```
Memory pressure detected → Non-critical processes frozen → Memory freed → System stable
```
**Benefit**: Controlled, predictable behavior with minimal disruption.

## Monitoring Status

### Startup Log

When memory pressure monitoring is enabled, you'll see:

```
└─ Memory Pressure: some 10.0%/30.0%, full 5.0%/15.0% (log|freeze)
...
Memory pressure monitoring enabled: some 10.0%/30.0%, full 5.0%/15.0%, actions: log/freeze
```

### Dashboard Display

The live dashboard shows real-time memory pressure status:

```
⚪ Memory Pressure: NONE (some: 0.0%, full: 0.0%, w:0/c:0)
🟢 Memory Pressure: LOW (some: 2.5%, full: 0.0%, w:0/c:0)
🟡 Memory Pressure: MEDIUM (some: 8.0%, full: 0.0%, w:1/c:0)
🟠 Memory Pressure: HIGH (some: 15.0%, full: 0.0%, w:3/c:0)
🔴 Memory Pressure: CRITICAL (some: 35.0%, full: 10.0%, w:5/c:2)
```

Status icons:
- ⚪ **NONE**: No pressure (0%)
- 🟢 **LOW**: Minimal pressure (0-5%)
- 🟡 **MEDIUM**: Noticeable pressure (5-10%)
- 🟠 **HIGH**: Significant pressure (10-15%)
- 🔴 **CRITICAL**: Severe pressure (>15% or any full stall)

Counters:
- `w:N` - Warning events count
- `c:N` - Critical events count

## Log Messages

### Warning Level
```
[WARN] WARNING memory pressure detected! some=12.50%, full=0.00% (thresholds: some=10.0%, full=5.0%)
[INFO] [Memory Pressure WARNING] Logging event
```

### Critical Level
```
[WARN] CRITICAL memory pressure detected! some=35.00%, full=8.50% (thresholds: some=30.0%, full=15.0%)
[INFO] [Memory Pressure CRITICAL] Freezing non-critical processes
[INFO] Froze Firefox process 12345
[INFO] Froze Brave process 67890
[INFO] Froze Telegram process 54321
[INFO] Memory pressure: froze 3 non-critical processes for 5 seconds
[INFO] Memory pressure: unfroze 3 processes
```

### Normalization
```
[DEBUG] Memory pressure normalized (some=2.00%, full=0.00%)
```

## Performance Impact

Memory pressure monitoring has minimal overhead:

- **CPU**: <0.01% (reads /proc file every 5 seconds)
- **Memory**: <100KB (simple data structures)
- **Latency**: ~1ms to read and parse PSI data

## Use Cases

### 1. Preventing OOM on Low-Memory Systems

**Scenario**: 8GB RAM system running multiple browsers
```toml
[memory_pressure]
enabled = true
some_threshold_warning = 5.0     # Lower threshold
some_threshold_critical = 15.0   # Lower threshold
action_critical = "freeze"       # Freeze browsers temporarily
```

### 2. Monitoring-Only Mode

**Scenario**: Just want to track memory pressure
```toml
[memory_pressure]
enabled = true
some_threshold_warning = 10.0
some_threshold_critical = 30.0
action_warning = "log"
action_critical = "log"          # Only log, no actions
```

### 3. Aggressive Protection

**Scenario**: Critical server, cannot afford OOM
```toml
[memory_pressure]
enabled = true
some_threshold_warning = 5.0     # Very sensitive
some_threshold_critical = 15.0
action_warning = "nice"          # Immediately lower priority
action_critical = "kill"         # Kill non-critical processes
```

## Limitations

1. **Requires Linux kernel 4.20+** with PSI support
2. **May freeze actively-used applications** (e.g., browser you're using)
3. **No per-application memory tracking** (system-wide only)
4. **Cannot prevent all OOM scenarios** (just reduces likelihood)

## Troubleshooting

### PSI Not Available
```
ERROR: Failed to read memory pressure: Other error: Failed to read /proc/pressure/memory: ...
```

**Solution**: Check kernel version and PSI support:
```bash
uname -r  # Should be >= 4.20
cat /proc/pressure/memory  # Should show pressure data
```

### Too Aggressive
If processes are being frozen too often:
```toml
some_threshold_critical = 50.0   # Increase threshold
action_critical = "log"          # Change to logging only
```

### Not Responding to Pressure
If memory fills up despite monitoring:
```toml
some_threshold_warning = 5.0     # Lower thresholds
some_threshold_critical = 15.0
action_critical = "kill"         # More aggressive action
check_interval_secs = 2          # Check more frequently
```

## Technical Details

### PSI Format
```
some avg10=12.50 avg60=8.32 avg300=5.12 total=1234567
full avg10=3.21 avg60=2.11 avg300=1.05 total=654321
```

- **avg10**: Average pressure over last 10 seconds (most responsive)
- **avg60**: Average over last 60 seconds
- **avg300**: Average over last 300 seconds (5 minutes)
- **total**: Total stall time in microseconds since boot

FreezR uses **avg10** for real-time responsiveness.

### Why Two Metrics?

- **some**: Indicates memory contention (at least one process waiting)
- **full**: Indicates severe memory starvation (ALL processes blocked)

A high `some` with low `full` means memory pressure exists but system is still functioning.
A high `full` means system is critically memory-starved.

## References

- [Linux PSI Documentation](https://www.kernel.org/doc/html/latest/accounting/psi.html)
- [Facebook's PSI Implementation](https://lwn.net/Articles/759658/)
- [Understanding Memory Pressure](https://facebookmicrosites.github.io/psi/docs/overview)

## Future Enhancements

- [ ] Per-process memory tracking
- [ ] ML-based pressure prediction
- [ ] Custom process priority lists
- [ ] Swap pressure monitoring
- [ ] CPU and I/O pressure monitoring

---

**Last Updated**: 2025-10-26
**Status**: Production Ready
