# FreezR Development Documentation

Documentation for contributors and developers.

## Available Documents

### [Architecture](ARCHITECTURE.md)
System design and technical architecture:
- High-level overview
- Component descriptions (freezr-core, freezr-daemon)
- Module breakdown
- Data flow
- Technology stack

### [Contributing Guide](CONTRIBUTING.md)
How to contribute to FreezR:
- Code of conduct
- Development setup
- Coding standards
- Pull request process
- Testing requirements

### [Code of Conduct](CODE_OF_CONDUCT.md)
Community guidelines and behavior expectations.

### [Roadmap](ROADMAP.md)
Project roadmap and planned features:
- Completed features
- In-progress work
- Planned improvements
- Long-term vision

### [Changelog](CHANGELOG.md)
Version history and release notes.

### [GitHub Description](GITHUB_DESCRIPTION.md)
Project description and marketing content for GitHub.

### [ML Process Analytics](ML_PROCESS_ANALYTICS.md)
Machine learning roadmap for process prediction:
- Pattern recognition goals
- Anomaly detection
- Historical data collection
- LSTM model planning

### [Implementation Summaries](IMPLEMENTATION_SUMMARY.md)
Detailed implementation notes:
- Feature implementation details
- Technical decisions
- Testing results

### [Snap Implementation](SNAP_IMPLEMENTATION_SUMMARY.md)
Snap/snapd monitoring implementation:
- Multi-core CPU detection
- Nice action implementation
- Violation tracking

## Project Structure

```
freezr/
├── crates/
│   ├── freezr-core/       # Core library (scanner, executor, systemd)
│   │   ├── src/
│   │   │   ├── scanner.rs      # Process scanning (315 lines)
│   │   │   ├── executor.rs     # Process termination (206 lines)
│   │   │   ├── systemd.rs      # Systemd integration (234 lines)
│   │   │   ├── types.rs        # Data structures (415 lines)
│   │   │   └── error.rs        # Error types (91 lines)
│   │   └── tests/             # Integration tests
│   │
│   ├── freezr-daemon/     # Main daemon
│   │   ├── src/
│   │   │   ├── main.rs         # CLI interface (253 lines)
│   │   │   ├── monitor.rs      # Monitoring logic (359 lines)
│   │   │   └── config.rs       # Configuration (399 lines)
│   │   └── lib.rs
│   │
│   ├── freezr-cli/        # CLI tool (planned)
│   └── freezr-gui/        # GUI application (planned)
│
├── config/examples/       # Example configurations
├── docs/                  # Documentation
└── logs/                  # Runtime logs
```

## Technology Stack

- **Language:** Rust 1.70+
- **Async Runtime:** Tokio
- **CLI Parsing:** Clap 4.x
- **Logging:** Tracing + Tracing-subscriber
- **Config:** TOML + Serde
- **Process Management:** nix crate (POSIX signals)
- **Testing:** Built-in Rust test framework

## Code Statistics

- **Total Lines:** 2,820 Rust code
- **Tests:** 72 (71 passed, 13 ignored)
- **Coverage:** ~85%
- **Modules:** 11
- **Dependencies:** Minimal (see Cargo.toml)

## Development Commands

```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Run all tests
cargo test --workspace

# Run specific test
cargo test --package freezr-core scanner

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Generate documentation
cargo doc --open

# Run with logging
RUST_LOG=debug cargo run --bin freezr-daemon -- watch
```

## Testing

### Unit Tests
Located in each module under `#[cfg(test)]`:
- `types.rs`: 17 tests
- `scanner.rs`: 11 tests
- `systemd.rs`: 5 tests
- `executor.rs`: 6 tests
- `error.rs`: 8 tests
- `config.rs`: 12 tests
- `monitor.rs`: 9 tests

### Integration Tests
Located in `crates/freezr-core/tests/`:
- `scanner_integration.rs`
- `systemd_integration.rs`

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific package
cargo test --package freezr-core

# Specific test
cargo test test_monitor_creation

# With output
cargo test -- --nocapture

# Ignored tests
cargo test -- --ignored
```

## Contributing Workflow

1. Fork repository
2. Create feature branch
3. Write code with tests
4. Run `cargo fmt` and `cargo clippy`
5. Ensure all tests pass
6. Submit pull request

See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## API Design Principles

1. **Type Safety** - Use Rust's type system to prevent errors
2. **Error Handling** - Comprehensive Result types, no panics
3. **Memory Safety** - No unsafe code
4. **Performance** - Zero-cost abstractions
5. **Testability** - Mock-friendly interfaces
6. **Documentation** - Doc comments for all public APIs

## Module Responsibilities

### freezr-core
Core library providing:
- Process scanning (`scanner.rs`)
- Process execution (`executor.rs`)
- Systemd integration (`systemd.rs`)
- Data types (`types.rs`)
- Error types (`error.rs`)

### freezr-daemon
Daemon binary providing:
- CLI interface (`main.rs`)
- Monitoring logic (`monitor.rs`)
- Configuration (`config.rs`)

### freezr-cli (planned)
CLI tool for:
- Manual process management
- Status queries
- Configuration validation

### freezr-gui (planned)
Desktop GUI for:
- Real-time monitoring
- Visual statistics
- Interactive control

## Quick Links

- [Architecture →](ARCHITECTURE.md)
- [Contributing →](CONTRIBUTING.md)
- [Roadmap →](ROADMAP.md)
- [← Back to Main Docs](../README.md)
