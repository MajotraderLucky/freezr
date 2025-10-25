# FreezR Architecture

## Table of Contents
- [Overview](#overview)
- [System Design](#system-design)
- [Core Components](#core-components)
- [Data Flow](#data-flow)
- [Process Management](#process-management)
- [Performance Considerations](#performance-considerations)
- [Security](#security)
- [Future Enhancements](#future-enhancements)

## Overview

FreezR is designed as a high-performance, low-overhead system daemon that prevents system freezes by proactively managing processes before they consume all available resources.

### Design Philosophy

1. **Proactive, not reactive** - Catch problems before system hangs
2. **Smart, not aggressive** - Freeze first, kill only when necessary
3. **Fast, not wasteful** - Minimal overhead (<0.5% CPU, ~3MB memory)
4. **Transparent, not hidden** - Log and notify all actions
5. **Configurable, not opinionated** - Adapt to user needs

### Key Differentiators

Unlike traditional resource managers:
- **Sub-second monitoring** (100-500ms) vs seconds/minutes
- **SIGSTOP/SIGCONT** for reversible freezing vs permanent killing
- **Direct /proc access** vs spawning external commands
- **Async parallel scanning** vs sequential blocking I/O

## System Design

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         FreezR System                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐        ┌──────────────┐                    │
│  │  CLI Tool   │◄──────►│   Daemon     │                    │
│  │ (freezr)    │  IPC   │(freezr-daemon)│                    │
│  └─────────────┘        └───────┬──────┘                    │
│                                  │                            │
│  ┌─────────────┐        ┌───────▼──────┐                    │
│  │  GUI App    │◄──────►│  Core Engine │                    │
│  │(freezr-gui) │  API   │(freezr-core) │                    │
│  └─────────────┘        └───────┬──────┘                    │
│                                  │                            │
│         ┌────────────────────────┼────────────────────┐      │
│         │                        │                    │      │
│    ┌────▼────┐          ┌────────▼─────┐      ┌──────▼────┐ │
│    │ Process │          │   Decision   │      │  Action   │ │
│    │ Scanner │─────────►│    Engine    │─────►│ Executor  │ │
│    └─────────┘  Stats   └──────────────┘      └───────────┘ │
│         │                       │                    │       │
│         │                       │                    │       │
└─────────┼───────────────────────┼────────────────────┼───────┘
          │                       │                    │
          │                       │                    │
     ┌────▼─────┐           ┌─────▼─────┐       ┌─────▼──────┐
     │  /proc   │           │ ML Model  │       │  Signals   │
     │filesystem│           │(optional) │       │ (SIGSTOP)  │
     └──────────┘           └───────────┘       └────────────┘
```

### Component Responsibilities

#### 1. Process Scanner (`scanner.rs`)
**Purpose:** Efficiently collect real-time process statistics

**Input:** None (scans entire system)
**Output:** `Vec<ProcessInfo>` with CPU, memory, and metadata

**Implementation:**
- Reads `/proc/[pid]/stat` directly (no external commands)
- Async parallel scanning with Tokio
- Calculates CPU percentage from utime/stime deltas
- Caches previous values for delta calculation

**Performance:**
- Scan all processes: ~5-10ms for 200 processes
- Memory usage: ~100 bytes per process
- Zero allocations in hot path (reuses buffers)

#### 2. Decision Engine (`engine.rs`)
**Purpose:** Determine which processes to freeze/kill based on rules and heuristics

**Input:** 
- Current process statistics
- System health metrics (load, temperature)
- Configuration rules

**Output:** `Vec<Action>` (freeze, kill, limit, restart)

**Decision Logic:**
```rust
pub enum Action {
    Freeze { pid: u32, duration: Duration },
    Unfreeze { pid: u32 },
    Kill { pid: u32, signal: Signal },
    LimitCgroup { pid: u32, cpu_quota: f64 },
    RestartService { name: String },
}

impl DecisionEngine {
    fn decide(&self, processes: &[ProcessInfo], health: &SystemHealth) -> Vec<Action> {
        // 1. Filter protected processes
        // 2. Calculate priority scores
        // 3. Check thresholds (CPU, memory, thermal)
        // 4. Apply ML predictions (if enabled)
        // 5. Generate actions
    }
}
```

**Priority System:**
```rust
enum ProcessPriority {
    Protected = 0,      // Never touch (systemd, sshd)
    System = 1,         // Only in emergency (kesl, NetworkManager)
    UserImportant = 2,  // Freeze when critical (terminals, IDE)
    UserNormal = 3,     // Freeze aggressively (browsers)
    Aggressive = 4,     // Kill first (node, electron with >90% CPU)
}
```

#### 3. Action Executor (`executor.rs`)
**Purpose:** Safely execute actions (freeze, kill, restart)

**Input:** `Vec<Action>` from decision engine
**Output:** `Vec<ActionResult>` (success/failure for each action)

**Safety Guarantees:**
- Never freeze PID 1 (init/systemd)
- Never freeze own process
- Verify process exists before action
- Handle EPERM gracefully (insufficient permissions)
- Rollback on critical errors

**Implementation:**
```rust
pub async fn execute(&self, actions: Vec<Action>) -> Vec<ActionResult> {
    let mut results = Vec::new();
    
    for action in actions {
        let result = match action {
            Action::Freeze { pid, .. } => {
                // Send SIGSTOP
                signal::kill(Pid::from_raw(pid as i32), Signal::SIGSTOP)?;
                ActionResult::Success
            },
            Action::Unfreeze { pid } => {
                // Send SIGCONT
                signal::kill(Pid::from_raw(pid as i32), Signal::SIGCONT)?;
                ActionResult::Success
            },
            // ... other actions
        };
        results.push(result);
    }
    
    results
}
```

## Data Flow

### Main Monitoring Loop

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut scanner = ProcessScanner::new();
    let engine = DecisionEngine::from_config(&config)?;
    let executor = ActionExecutor::new();
    
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    
    loop {
        interval.tick().await;
        
        // 1. Scan all processes (parallel, async)
        let processes = scanner.scan_all().await?;
        
        // 2. Check system health
        let health = SystemHealth::current()?;
        
        // 3. Make decisions
        let actions = engine.decide(&processes, &health);
        
        // 4. Execute actions
        let results = executor.execute(actions).await;
        
        // 5. Log results
        for result in results {
            tracing::info!("Action executed: {:?}", result);
        }
    }
}
```

### Detailed Flow Diagram

```
┌──────────────┐
│  Start Loop  │
└──────┬───────┘
       │
       ▼
┌──────────────────────────────┐
│  Scan Processes (Parallel)   │ 
│  - Read /proc/[pid]/stat     │  ← 5-10ms
│  - Calculate CPU %           │
│  - Get memory usage          │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│  Check System Health         │
│  - Load average              │  ← 1-2ms
│  - CPU temperature           │
│  - Available memory          │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│  Decision Engine             │
│  - Filter protected          │  ← 1ms
│  - Calculate priorities      │
│  - Check thresholds          │
│  - Generate actions          │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│  Execute Actions             │
│  - Send signals              │  ← <1ms per action
│  - Update cgroups            │
│  - Restart services          │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│  Log & Notify                │
│  - Write to log file         │  ← ~1ms
│  - Send notifications        │
└──────┬───────────────────────┘
       │
       ▼
┌──────────────────────────────┐
│  Sleep (500ms interval)      │  ← Configurable
└──────┬───────────────────────┘
       │
       └──────► Loop back to start
```

### Total Loop Time Budget

| Phase | Time | Percentage |
|-------|------|------------|
| Process Scanning | 5-10ms | 1-2% |
| Health Check | 1-2ms | 0.2-0.4% |
| Decision Making | 1ms | 0.2% |
| Action Execution | <1ms | <0.2% |
| Logging | ~1ms | 0.2% |
| **Active Work** | **~10-15ms** | **2-3%** |
| Sleep | 485-490ms | 97-98% |
| **Total** | **500ms** | **100%** |

**CPU Usage:** 2-3% of one monitoring cycle = **0.4-0.6% average CPU**

## Process Management

### State Machine

Each monitored process can be in one of these states:

```
┌─────────┐
│ Running │◄──────┐
└────┬────┘       │
     │            │
     │ CPU > 85%  │ System
     │ (3 times)  │ recovered
     ▼            │
┌─────────┐       │
│ Frozen  │───────┘
│(SIGSTOP)│       
└────┬────┘       
     │            
     │ Still high
     │ after 5s   
     ▼            
┌─────────┐       
│ Killed  │       
│(SIGTERM)│       
└─────────┘       
```

### Freeze Strategy

**When to freeze:**
1. CPU usage > 85% for 3 consecutive checks (~1.5 seconds)
2. OR system load average > 10
3. AND process is not protected
4. AND not already frozen

**Freeze duration:**
- Default: 2 seconds
- Configurable per-process or global
- Auto-unfreeze when system recovers

**Example:**
```toml
[[processes.rules]]
name_pattern = "chrome"
cpu_threshold = 80.0
action = "freeze"
freeze_duration = "3s"
max_freezes_per_minute = 5
```

### Kill Strategy

**When to kill:**
1. Process frozen >5 times in 1 minute
2. OR CPU > 95% for 10 consecutive checks (~5 seconds)
3. OR memory > 8GB and swapping system
4. AND not protected

**Kill escalation:**
1. SIGTERM (graceful shutdown)
2. Wait 5 seconds
3. SIGKILL (force kill) if still running

### Service Restart Strategy

For systemd services (like KESL):
```rust
Action::RestartService { name: "kesl" }
  ↓
1. sudo systemctl daemon-reload
2. sudo systemctl restart kesl
3. Verify service started
4. Apply cgroup limits
```

## Performance Considerations

### Memory Management

**Buffer Reuse:**
```rust
pub struct ProcessScanner {
    // Reused buffer to avoid allocations
    buffer: Vec<ProcessInfo>,
    // Previous stats for delta calculation
    prev_stats: HashMap<u32, ProcStat>,
}
```

**Arena Allocation:**
```rust
// Use typed-arena for temporary process data
let arena = Arena::new();
for pid in pids {
    let stat = arena.alloc(read_proc_stat(pid)?);
    // Process stat...
}
// All memory freed at once when arena drops
```

### CPU Optimization

**Parallel Scanning:**
```rust
// Scan all PIDs in parallel with tokio
let handles: Vec<_> = pids
    .into_iter()
    .map(|pid| tokio::spawn(async move {
        read_process_stats(pid).await
    }))
    .collect();

let results = futures::future::join_all(handles).await;
```

**SIMD for Calculations:**
```rust
// Use SIMD for batch CPU calculations (future optimization)
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
```

### I/O Optimization

**Async File Reading:**
```rust
// Tokio async file I/O for /proc
let mut file = tokio::fs::File::open(path).await?;
let mut buffer = Vec::new();
file.read_to_end(&mut buffer).await?;
```

**Batched System Calls:**
```rust
// Freeze multiple processes in one batch
for pid in pids_to_freeze {
    signal::kill(Pid::from_raw(pid), Signal::SIGSTOP)?;
}
// Much faster than individual CLI calls
```

## Security

### Privilege Separation

**Daemon runs as root** (required for sending signals to all processes)

**CLI runs as user** (communicates via socket)

```
User                Root
┌─────┐            ┌──────────┐
│ CLI │────────────►│  Daemon  │
└─────┘   Unix     └──────────┘
         Socket         │
                        ▼
                   Executes with
                   root privileges
```

### Protections

1. **Never freeze critical processes:**
```rust
const PROTECTED_NAMES: &[&str] = &[
    "systemd", "init", "sshd", "NetworkManager"
];
```

2. **Never freeze own process:**
```rust
let own_pid = std::process::id();
if pid == own_pid {
    return Err(Error::CannotFreezeOwnProcess);
}
```

3. **Validate configuration:**
```rust
// Don't allow rules that could brick system
if rule.action == Action::Kill && is_critical_process(&rule.name) {
    return Err(Error::DangerousConfiguration);
}
```

4. **Audit logging:**
```rust
tracing::warn!(
    pid = %pid,
    process = %name,
    action = "freeze",
    reason = "cpu_threshold_exceeded",
    "Freezing process"
);
```

## Future Enhancements

### Phase 2: Machine Learning

```rust
pub struct MLPredictor {
    model: LSTMModel,
    history: RingBuffer<ProcessStats>,
}

impl MLPredictor {
    /// Predict if process will spike CPU in next 5 seconds
    pub fn predict_spike(&self, pid: u32) -> f64 {
        let features = self.extract_features(pid);
        self.model.predict(&features)
    }
}
```

**Training data:**
- CPU usage over time (1-minute windows)
- Memory growth rate
- I/O patterns
- Time of day (some processes spike at specific times)

**Prediction:**
- Probability of spike in next 5 seconds
- If >80% probability and current CPU >50% → preemptive cgroup limit

### Phase 3: Cgroup Integration

```rust
pub async fn limit_cgroup(&self, pid: u32, cpu_percent: f64) -> Result<()> {
    // Move process to dedicated cgroup
    let cgroup_path = format!("/sys/fs/cgroup/freezr/{}", pid);
    std::fs::create_dir_all(&cgroup_path)?;
    
    // Set CPU quota (50% = 50000/100000)
    let quota = (cpu_percent * 1000.0) as u64;
    std::fs::write(
        format!("{}/cpu.max", cgroup_path),
        format!("{} 100000", quota)
    )?;
    
    // Move process
    std::fs::write(
        format!("{}/cgroup.procs", cgroup_path),
        pid.to_string()
    )?;
    
    Ok(())
}
```

**Advantages:**
- Smooth CPU limiting vs freeze/unfreeze
- Process continues running, just slower
- No visible interruption to user

### Phase 4: Distributed Monitoring

For enterprise deployments:

```
┌────────────┐     ┌────────────┐     ┌────────────┐
│  Server 1  │     │  Server 2  │     │  Server 3  │
│  FreezR    │     │  FreezR    │     │  FreezR    │
└──────┬─────┘     └──────┬─────┘     └──────┬─────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          │
                    ┌─────▼──────┐
                    │  Central   │
                    │ Dashboard  │
                    └────────────┘
```

**Features:**
- Centralized configuration management
- Aggregated metrics and alerts
- Coordinated actions (if one server high load, migrate processes)

---

## References

- [Linux /proc documentation](https://www.kernel.org/doc/Documentation/filesystems/proc.txt)
- [systemd cgroup integration](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)
- [Process signals](https://man7.org/linux/man-pages/man7/signal.7.html)
- [Tokio async runtime](https://tokio.rs/tokio/tutorial)
