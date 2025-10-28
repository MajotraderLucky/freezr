# Changelog

All notable changes to FreezR will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added - OOM Prevention System (2025-10-28)

#### Neovim Memory Monitoring
- **Feature**: Memory-based monitoring for Neovim processes
- **Scanner**: Added `scan_nvim_processes()` and `find_nvim_pids()` in `scanner.rs`
- **Detection**: Identifies nvim processes by command pattern (nvim, /usr/bin/nvim, etc.)
- **Threshold**: Configurable memory limit (default: 1GB)
- **Configuration**:
  ```toml
  [nvim]
  memory_threshold_mb = 1024
  enabled = true
  ```

#### Priority-Based OOM Prevention
- **Feature**: Intelligent process termination order during critical memory pressure
- **Implementation**: Priority-based killing in `kill_non_critical_processes()` (monitor.rs:1464-1573)
- **Kill Order**:
  1. 🔴 Priority 1: Brave browser (sacrificable, 1-2GB freed)
  2. 🟠 Priority 2: Telegram messenger (less critical, 300-500MB freed)
  3. 🟡 Priority 3: Neovim (only if >1GB, last resort, 1-2GB freed)
  4. 🔵 Priority 4: Firefox (additional protection)
- **Memory Pressure Action**: Changed from "freeze" to "kill" for proactive OOM prevention
- **Configuration**:
  ```toml
  [memory_pressure]
  action_critical = "kill"  # Changed from "freeze"
  ```

#### Detailed OOM Event Logging
- **Feature**: Comprehensive forensics for every OOM prevention event
- **What's Logged**:
  - Current PSI metrics (some/full avg10/avg60/avg300)
  - System memory state (MemTotal/MemAvailable from /proc/meminfo)
  - Top 10 memory consumers sorted by RAM usage
  - Each process: Type, PID, RAM (MB), CPU%, full command (60 chars)
  - Kill priority with emoji markers (🔴🟠🟡🔵)
  - Total processes killed and memory freed
- **Log Format**:
  ```
  ╔═══════════════════════════════════════════════════════════╗
  ║           🚨 CRITICAL MEMORY PRESSURE DETECTED 🚨         ║
  ╚═══════════════════════════════════════════════════════════╝
  PSI Metrics: some=35.20%, full=18.50%
  PSI Averages: some(10s/60s/300s)=35.20/28.40/22.10%
  System Memory: MemTotal: 16GB | MemAvailable: 1.2GB

  === OOM Prevention: Analyzing memory consumers ===
  Top memory consumers before OOM prevention:
    #1 nvim PID:6161 RAM:2345MB CPU:15.3% CMD:/usr/bin/nvim
    #2 Brave PID:88478 RAM:1856MB CPU:3.0% CMD:brave --type=renderer

  🔴 [Priority 1] Killing Brave PID:88478 RAM:1856MB
  🟡 [Priority 3] Killing nvim PID:6161 RAM:2345MB

  === OOM Prevention completed: killed 2 processes, freed 4201MB ===
  ```
- **Implementation**:
  - Enhanced PSI logging in `check_memory_pressure()` (monitor.rs:1282-1315)
  - Detailed process analysis in `kill_non_critical_processes()` (monitor.rs:1464-1573)

#### Documentation Updates
- **README.md**: Added "🛡️ OOM Prevention System" section with:
  - PSI mechanism explanation
  - Priority-based killing strategy
  - Detailed logging examples
  - Pattern analysis guide
- **CLAUDE.md**: Added comprehensive OOM prevention documentation with:
  - How FreezR outpaces OOM killer using PSI
  - Timeline visualization
  - Real-world scenario examples
  - Log viewing and analysis commands

### Technical Details

#### Files Modified
- `crates/freezr-core/src/scanner.rs`: Added nvim scanner (lines 302-341)
- `crates/freezr-daemon/src/monitor.rs`:
  - Enhanced OOM logging (lines 1282-1315)
  - Priority-based killing (lines 1464-1573)
- `freezr.toml`:
  - Added [nvim] section
  - Changed memory_pressure.action_critical to "kill"
- `README.md`: Added OOM Prevention System section
- `CLAUDE.md`: Added detailed PSI and OOM documentation

#### Build Information
- Compiled successfully with Rust 1.70+
- Compilation time: ~40-44 seconds
- Binary size: ~3MB (optimized release build)
- Warnings: 4 unused functions in cgroups/utils.rs (non-critical)

#### Systemd Integration
- Service file created: `~/.config/systemd/user/freezr.service`
- Auto-restart enabled: RestartSec=10
- Resource limits: MemoryMax=100M, CPUQuota=10%
- Status: Active and running
- Logs: `journalctl --user -u freezr.service`

### Why This Matters

**Before FreezR OOM Prevention:**
```
nvim grows to 5GB → System swap thrashes → Everything freezes →
OOM killer activates → Kills random process (maybe plasmashell!) →
Black screen 😱
```

**With FreezR OOM Prevention:**
```
nvim grows to 5GB → PSI detects pressure at 15% full →
FreezR kills Brave (2GB freed) → FreezR kills Telegram (300MB freed) →
FreezR kills nvim (5GB freed) → System recovers → OOM killer never activates ✅
```

**Key Advantages:**
- **Proactive**: Acts at 15% memory pressure, not 100% exhaustion
- **Intelligent**: Kills less-critical processes first
- **Fast**: 5-second detection interval
- **Forensic**: Complete logs for post-mortem analysis
- **Configurable**: Adjust thresholds and priorities per use case

---

## [0.2.0] - D-Bus Migration (2025-10-27)

See [CHANGELOG_DBUS.md](CHANGELOG_DBUS.md) for complete D-Bus migration details.

---

## [0.1.0] - Initial Release

### Added
- Basic process monitoring (KESL, Node.js, Snap)
- CPU-based freeze/kill strategies
- Extended statistics and dashboard
- Systemd integration
- TOML configuration
