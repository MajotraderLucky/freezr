# Technical Documentation

In-depth technical documentation for FreezR implementation, features, and system integration.

## Available Documentation

### System Integration

#### [Systemd Service](SYSTEMD_SERVICE.md)
Complete guide to FreezR systemd service:
- Native Rust implementation
- CLI subcommands (install/uninstall/status)
- Service configuration and management
- Auto-start on boot
- Resource limits and security
- Logging integration

**Essential for production deployment!**

### Monitoring Features

#### [Memory Pressure Monitoring](MEMORY_PRESSURE.md)
PSI-based proactive OOM prevention:
- Linux Pressure Stall Information (PSI)
- Two-tier warning/critical thresholds
- Proactive actions (log, nice, freeze, kill)
- Early warning before system freezes
- Dashboard integration
- Configuration guide

**Prevents OOM situations before they occur!**

#### [Snap/Snapd Monitoring](SNAP_MONITORING.md)
Handling resource-heavy snap processes:
- Multi-core CPU threshold (>300%)
- Nice action (priority reduction)
- Violation tracking
- Configuration examples
- Integration with main monitoring loop

**Controls snap processes without killing them!**

### Operational

#### [Log Maintenance](LOG_MAINTENANCE.md)
Complete log lifecycle management:
- Daily rotation with tracing-appender
- Archive and compression scripts
- Automatic cleanup
- Cron automation
- Dashboard statistics integration
- Disk space management

**Keep logs organized and manageable!**

## Quick Reference

### When to Read What

**Setting up production deployment?**
→ Start with [Systemd Service](SYSTEMD_SERVICE.md)

**System experiencing OOM issues?**
→ Read [Memory Pressure Monitoring](MEMORY_PRESSURE.md)

**Snap processes consuming too much CPU?**
→ Check [Snap Monitoring](SNAP_MONITORING.md)

**Logs growing too large?**
→ Review [Log Maintenance](LOG_MAINTENANCE.md)

## Implementation Status

| Feature | Status | Documentation |
|---------|--------|---------------|
| Systemd Service | ✅ Complete | SYSTEMD_SERVICE.md |
| Memory Pressure | ✅ Complete | MEMORY_PRESSURE.md |
| Snap Monitoring | ✅ Complete | SNAP_MONITORING.md |
| Log Maintenance | ✅ Complete | LOG_MAINTENANCE.md |
| Thermal Monitoring | 🚧 Planned | - |
| Disk I/O Monitoring | 🚧 Planned | - |
| Network Monitoring | 🚧 Planned | - |

## Related Documentation

- [User Guide](../user-guide/README.md) - End-user documentation
- [Development Guide](../development/README.md) - Contributing and architecture
- [API Documentation](../api/README.md) - Code reference

## Contributing

Found an issue or want to improve documentation?
See [CONTRIBUTING.md](../development/CONTRIBUTING.md)
