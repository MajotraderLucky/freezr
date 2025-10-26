# Cgroup v2 Integration Research

## System Analysis

### Current Setup
- **Cgroup Version**: v2 (unified hierarchy)
- **Mount Point**: `/sys/fs/cgroup`
- **Available Controllers**: cpuset, cpu, io, memory, hugetlb, pids, rdma, misc, dmem

### Cgroup v2 Architecture

**Unified Hierarchy** - all controllers use single tree structure:
```
/sys/fs/cgroup/
├── user.slice/
│   ├── user-1000.slice/
│   │   └── user@1000.service/
│   │       └── app.slice/
│   │           └── app-kitty-*.scope
├── system.slice/
│   ├── kesl.service/
│   └── other services...
└── custom cgroups (can be created)
```

## Key Controller Files

### CPU Controller

**cpu.max** - CPU bandwidth limit
```
Format: "$MAX $PERIOD" (microseconds)
Example: "50000 100000" = 50% CPU (50ms out of 100ms)
Default: "max 100000" = unlimited
```

**cpu.weight** - Relative CPU share
```
Range: 1-10000
Default: 100
Higher = more CPU time when contention
```

**cpu.stat** - Usage statistics
```
Fields:
- usage_usec: Total CPU time consumed
- user_usec: User-space CPU time
- system_usec: Kernel-space CPU time
- nr_periods: Number of enforcement periods
- nr_throttled: Number of throttled periods
- throttled_usec: Total throttled time
```

### Memory Controller

**memory.max** - Hard memory limit
```
Format: bytes or "max"
Example: "536870912" = 512MB
Kill process (OOM) if exceeded
```

**memory.high** - Soft memory limit
```
Format: bytes or "max"
Example: "268435456" = 256MB
Throttle (slowdown) if exceeded, but don't kill
```

**memory.current** - Current usage
```
Format: bytes
Read-only
```

**memory.pressure** - Memory pressure metrics (PSI)
```
Format: "some avg10=X avg60=Y avg300=Z total=T"
        "full avg10=X avg60=Y avg300=Z total=T"
```

### IO Controller

**io.max** - IO bandwidth limit
```
Format: "$MAJ:$MIN rbps=$RBPS wbps=$WBPS riops=$RIOPS wiops=$WIOPS"
Example: "8:0 rbps=2097152 wbps=1048576" = 2MB/s read, 1MB/s write
```

## Process to Cgroup Mapping

Each process belongs to exactly one cgroup:

```bash
# Find process cgroup
cat /proc/$PID/cgroup
# Output: 0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-*.scope

# Move process to cgroup
echo $PID > /sys/fs/cgroup/path/to/cgroup/cgroup.procs
```

## FreezR Integration Strategy

### Option 1: Per-Process Cgroups (Dynamic)

**Approach**: Create temporary cgroup for each monitored process

**Pros**:
- Fine-grained control per process
- Can limit specific PIDs without affecting others
- Easy cleanup when process exits

**Cons**:
- Requires root/CAP_SYS_ADMIN
- More complex to manage
- Overhead of creating/destroying cgroups

**Implementation**:
```
/sys/fs/cgroup/freezr.slice/
├── process-$PID-1/
│   ├── cgroup.procs (contains PID 1)
│   ├── cpu.max
│   └── memory.max
└── process-$PID-2/
    ├── cgroup.procs
    ├── cpu.max
    └── memory.max
```

### Option 2: Shared Cgroups (Static)

**Approach**: Pre-create cgroups for process categories

**Pros**:
- Simpler management
- Less overhead
- Can set limits once

**Cons**:
- Less granular (affects all processes in category)
- Harder to track individual processes
- May limit good processes along with bad ones

**Implementation**:
```
/sys/fs/cgroup/freezr.slice/
├── high-cpu/         # Processes using >80% CPU
│   ├── cpu.max = "30000 100000"  # 30% limit
│   └── cgroup.procs
├── high-memory/      # Processes using >2GB RAM
│   ├── memory.max = "2147483648"  # 2GB limit
│   └── cgroup.procs
└── throttled/        # Processes being throttled
    ├── cpu.max = "10000 100000"  # 10% limit
    ├── memory.max = "536870912"  # 512MB limit
    └── cgroup.procs
```

### Option 3: Hybrid (Recommended)

**Approach**: Combine both strategies

1. **Static cgroups** for known process types:
   - `freezr.slice/kesl/` - Kaspersky with fixed limits
   - `freezr.slice/browsers/` - Firefox/Chrome with shared limits
   - `freezr.slice/development/` - Node/Python with higher limits

2. **Dynamic cgroups** for temporary throttling:
   - Create when violation detected
   - Apply limits
   - Remove when process exits or CPU normalizes

**Benefits**:
- Best of both worlds
- Efficient for known processes
- Flexible for unknown/temporary issues

## Required Permissions

### CAP_SYS_ADMIN capability OR root

**Methods**:
1. Run daemon as root (systemd service)
2. Use capabilities: `setcap cap_sys_admin+ep /path/to/freezr-daemon`
3. Use systemd delegation (recommended)

### Systemd Delegation (Safest)

Create service with `Delegate=yes`:
```ini
[Service]
Type=notify
ExecStart=/usr/local/bin/freezr-daemon
Delegate=yes
DelegateSubgroup=freezr
```

Allows daemon to manage cgroups under its own subtree without full root.

## Rust Crates for Cgroup Management

### 1. **controlgroup** (Recommended)
```toml
controlgroup = "1.2"
```
- Idiomatic Rust API
- Supports cgroup v2
- Well-maintained
- Easy to use

### 2. **libcgroup-rs**
```toml
libcgroup = "0.1"
```
- Bindings to libcgroup C library
- Requires system library
- More low-level

### 3. **Manual /sys/fs/cgroup access**
- Direct file operations
- No dependencies
- Full control but more code

**Recommendation**: Start with `controlgroup` crate for simplicity.

## Implementation Plan

### Phase 1: Basic Structure (Day 1-2)

1. Create `crates/freezr-core/src/cgroups.rs`
2. Define API:
   ```rust
   pub struct CgroupManager {
       root_path: PathBuf,
   }

   impl CgroupManager {
       pub fn new(root_path: PathBuf) -> Result<Self>;
       pub fn create_cgroup(&self, name: &str) -> Result<Cgroup>;
       pub fn set_cpu_limit(&self, cgroup: &Cgroup, percent: f64) -> Result<()>;
       pub fn set_memory_limit(&self, cgroup: &Cgroup, bytes: u64) -> Result<()>;
       pub fn add_process(&self, cgroup: &Cgroup, pid: u32) -> Result<()>;
       pub fn remove_cgroup(&self, cgroup: &Cgroup) -> Result<()>;
   }
   ```

3. Add to `freezr-core/src/lib.rs`:
   ```rust
   pub mod cgroups;
   pub use cgroups::CgroupManager;
   ```

### Phase 2: CPU Quota Implementation (Day 3)

1. Implement `set_cpu_limit()`:
   - Convert percentage to microseconds
   - Write to `cpu.max` file
   - Validate limits (1-100%)

2. Add CPU statistics reading:
   - Parse `cpu.stat`
   - Track throttling events
   - Expose metrics

### Phase 3: Memory Limits Implementation (Day 4)

1. Implement `set_memory_limit()`:
   - Write to `memory.max`
   - Support both hard and soft limits
   - Handle OOM scenarios

2. Add memory statistics:
   - Read `memory.current`
   - Parse `memory.pressure` (PSI)
   - Expose metrics

### Phase 4: Integration (Day 5-6)

1. Update TOML configuration:
   ```toml
   [cgroups]
   enabled = true
   root_path = "/sys/fs/cgroup/freezr.slice"
   strategy = "hybrid"  # static, dynamic, hybrid

   [[cgroups.static_groups]]
   name = "kesl"
   cpu_limit_percent = 30
   memory_limit_mb = 512
   processes = ["kesl"]

   [[cgroups.static_groups]]
   name = "browsers"
   cpu_limit_percent = 200  # Allow 2 cores
   memory_limit_mb = 4096
   processes = ["firefox", "brave", "chrome"]
   ```

2. Integrate with monitor loop:
   - Check if cgroups enabled
   - Apply static limits on startup
   - Create dynamic cgroups for violations
   - Cleanup on shutdown

### Phase 5: Testing (Day 7)

1. Unit tests:
   - Cgroup creation/deletion
   - Limit setting/reading
   - Process assignment

2. Integration tests:
   - End-to-end with real processes
   - Verify CPU throttling
   - Verify memory limits
   - Cleanup verification

3. Manual testing:
   - Run high-CPU process
   - Verify cgroup limits applied
   - Check systemd integration

## Security Considerations

1. **Privilege Escalation**: Only allow cgroup operations in controlled subtree
2. **Resource Exhaustion**: Limit number of dynamic cgroups
3. **Permission Checks**: Validate before moving processes
4. **Cleanup**: Ensure cgroups removed on exit
5. **Audit Logging**: Log all cgroup operations

## Testing Strategy

### Manual Testing Commands

```bash
# Create test cgroup
sudo mkdir /sys/fs/cgroup/freezr-test
echo "+cpu +memory" | sudo tee /sys/fs/cgroup/cgroup.subtree_control

# Set CPU limit (30%)
echo "30000 100000" | sudo tee /sys/fs/cgroup/freezr-test/cpu.max

# Set memory limit (512MB)
echo "536870912" | sudo tee /sys/fs/cgroup/freezr-test/memory.max

# Move process to cgroup
echo $PID | sudo tee /sys/fs/cgroup/freezr-test/cgroup.procs

# Verify limits applied
cat /sys/fs/cgroup/freezr-test/cpu.stat
cat /sys/fs/cgroup/freezr-test/memory.current

# Cleanup
sudo rmdir /sys/fs/cgroup/freezr-test
```

### Stress Testing

```bash
# CPU stress
stress-ng --cpu 4 --timeout 60s &
PID=$!

# Monitor with FreezR applying limits
# Verify CPU usage stays under limit

# Memory stress
stress-ng --vm 1 --vm-bytes 1G --timeout 60s &
PID=$!

# Verify memory limit enforced
```

## References

- [Cgroup v2 Documentation](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
- [Control Group v2 (freedesktop)](https://systemd.io/CGROUP_DELEGATION/)
- [controlgroup crate](https://docs.rs/controlgroup/)
- [Linux Pressure Stall Information (PSI)](https://www.kernel.org/doc/html/latest/accounting/psi.html)

## Next Steps

1. ✅ Research completed
2. ⏭️ Design module API (cgroups.rs)
3. ⏭️ Implement basic cgroup operations
4. ⏭️ Add CPU quota management
5. ⏭️ Add memory limits
6. ⏭️ Integrate with config
7. ⏭️ Integrate with monitor
8. ⏭️ Write tests
9. ⏭️ Update documentation
