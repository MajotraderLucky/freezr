# FreezR Documentation

Welcome to FreezR documentation! This guide will help you understand, install, configure, and use FreezR effectively.

## 📚 Documentation Structure

### User Guide
Getting started and day-to-day usage:
- [Getting Started](user-guide/getting-started.md) - Installation and quick start
- Configuration Guide (coming soon)
- Troubleshooting Guide (coming soon)

### Deployment
Production deployment guides:
- [Systemd Integration](deployment/systemd.md) - Running FreezR as a system service
- Docker Deployment (planned)
- Kubernetes Deployment (planned)

### Examples
Real-world usage scenarios:
- [Common Scenarios](examples/common-scenarios.md) - Typical use cases and configurations

### Development
For contributors and developers:
- [Architecture](ARCHITECTURE.md) - System design and components
- API Documentation (planned)
- Contributing Guide (planned)

### API Reference
Technical API documentation:
- Rust API (planned)
- Configuration Schema (planned)

## 🚀 Quick Links

### New Users
1. [Installation](user-guide/getting-started.md#installation)
2. [Quick Start](user-guide/getting-started.md#quick-start)
3. [Configuration](user-guide/getting-started.md#configuration)

### System Administrators
1. [Systemd Service Setup](deployment/systemd.md)
2. [Production Best Practices](examples/common-scenarios.md#best-practices)
3. [Log Management](user-guide/getting-started.md#viewing-logs)

### Developers
1. [Architecture Overview](ARCHITECTURE.md)
2. Building from Source (see README.md)
3. Running Tests (see README.md)

## 📖 What is FreezR?

FreezR is a high-performance system resource guardian written in Rust that prevents system freezes by intelligently monitoring and managing runaway processes.

**Key Features:**
- ⚡ Ultra-fast monitoring with minimal overhead (<0.5% CPU, ~3MB RAM)
- 🧊 Smart process management (freeze, kill, restart services)
- 🎯 KESL antivirus monitoring with automatic restart
- 🔥 Node.js hung process detection and auto-kill
- 📊 Comprehensive logging and statistics
- ⚙️ TOML-based configuration
- 🔐 Type-safe, memory-safe Rust implementation
- ✅ 85% test coverage with 72 automated tests

## 🎯 Common Use Cases

### 1. KESL Antivirus Management
Monitor and automatically restart KESL service when it exceeds resource limits.

**See:** [KESL Only Scenario](examples/common-scenarios.md#scenario-1-monitoring-kesl-service)

### 2. Node.js Process Monitoring
Detect and kill hung Node.js processes that consume excessive CPU.

**See:** [Node.js Monitoring](examples/common-scenarios.md#scenario-2-monitoring-nodejs-processes)

### 3. Production Server Protection
Aggressive monitoring to prevent any system freezes.

**See:** [Production Setup](examples/common-scenarios.md#scenario-5-aggressive-protection-production-server)

### 4. Development Environment
Catch runaway builds and npm processes without interfering with normal development.

**See:** [Development Setup](examples/common-scenarios.md#scenario-6-development-environment)

## 📊 Performance Comparison

| Metric | Bash Script | FreezR (Rust) |
|--------|------------|---------------|
| CPU Usage | 5-10% | <0.5% |
| Memory | ~20MB | ~3MB |
| Type Safety | None | Strong |
| Error Handling | Basic | Comprehensive |
| Tests | Manual | 72 automated |
| Performance | 1x | 10-20x faster |

## 🛠️ Installation Methods

### From Source
```bash
cd /home/ryazanov/.myBashScripts/freezr
cargo build --release
./target/release/freezr-daemon --help
```

### System-wide Installation
```bash
sudo cp target/release/freezr-daemon /usr/local/bin/
freezr-daemon --version
```

### Systemd Service
See [Systemd Integration Guide](deployment/systemd.md)

## 📝 Basic Usage

### Single Check
```bash
freezr-daemon monitor
```

### Continuous Monitoring
```bash
freezr-daemon watch
```

### With Custom Config
```bash
freezr-daemon --config /path/to/config.toml watch
```

### Generate Config
```bash
freezr-daemon generate-config --output config.toml
```

## 🔧 Configuration Overview

FreezR uses TOML configuration files:

```toml
[kesl]
cpu_threshold = 30.0          # CPU limit (%)
memory_threshold_mb = 600     # Memory limit (MB)
max_violations = 3            # Max violations before restart

[node]
cpu_threshold = 80.0          # Node.js CPU limit (%)
auto_kill = true              # Auto-kill hung processes

[monitoring]
check_interval_secs = 3       # Check interval
min_restart_interval_secs = 100  # Min time between restarts
```

**See:** [Configuration Guide](user-guide/getting-started.md#configuration)

## 📂 Project Structure

```
freezr/
├── crates/
│   ├── freezr-core/       # Core library (scanner, executor, systemd)
│   ├── freezr-daemon/     # Main daemon (monitor, config, CLI)
│   ├── freezr-cli/        # CLI tool (planned)
│   └── freezr-gui/        # GUI application (planned)
├── docs/
│   ├── user-guide/        # User documentation
│   ├── deployment/        # Deployment guides
│   ├── examples/          # Usage examples
│   ├── development/       # Developer docs
│   └── api/               # API reference
├── config/
│   └── examples/          # Example configurations
└── logs/                  # Runtime logs
```

## 🐛 Troubleshooting

### KESL Not Found
```bash
sudo systemctl start kesl
```

### Permission Denied
```bash
sudo freezr-daemon watch
```

### Config Not Found
```bash
freezr-daemon generate-config
```

**See:** [Troubleshooting Guide](user-guide/getting-started.md#troubleshooting)

## 📞 Support

- **Documentation:** This docs directory
- **Logs:** `./logs/freezr-daemon.log.*`
- **Issues:** Report in project repository
- **Architecture:** See [ARCHITECTURE.md](ARCHITECTURE.md)

## 🗺️ Roadmap

### Implemented ✅
- KESL process monitoring (CPU, memory)
- Node.js process monitoring and auto-kill
- Violation counter system
- Automatic service restart
- TOML configuration
- File logging with rotation
- CLI interface (monitor, watch, force-restart)

### Planned 🚧
- Web dashboard
- Desktop notifications
- ML-based predictions
- Thermal monitoring
- Process freezing (SIGSTOP/SIGCONT)
- Custom rules engine
- Plugin system
- Multi-service support

## 📄 License

FreezR is dual-licensed under MIT and Apache 2.0.

See [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE) for details.

## 🙏 Contributing

Contributions are welcome! See the development documentation for guidelines.

---

**Quick Navigation:**
- [← Back to Main README](../README.md)
- [Getting Started →](user-guide/getting-started.md)
- [Systemd Deployment →](deployment/systemd.md)
- [Examples →](examples/common-scenarios.md)
