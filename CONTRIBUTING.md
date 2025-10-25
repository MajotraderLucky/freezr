# Contributing to FreezR

First off, thank you for considering contributing to FreezR! 🎉

FreezR is an open source project and we love to receive contributions from our community. There are many ways to contribute, from writing tutorials or blog posts, improving the documentation, submitting bug reports and feature requests, or writing code which can be incorporated into FreezR itself.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the existing issues as you might find out that you don't need to create one. When you are creating a bug report, please include as many details as possible:

**Bug Report Template:**
```markdown
**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Start FreezR with config '...'
2. Run process '...'
3. See error

**Expected behavior**
What you expected to happen.

**Actual behavior**
What actually happened.

**Environment:**
 - OS: [e.g., Ubuntu 22.04]
 - Kernel: [e.g., 6.2.0]
 - FreezR version: [e.g., 0.1.0]
 - Rust version: [e.g., 1.70.0]

**Logs**
```
Paste relevant logs here
```

**Additional context**
Any other information that might be relevant.
```

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, please include:

- **Clear and descriptive title**
- **Detailed description** of the proposed functionality
- **Use cases** - why would this be useful?
- **Possible implementation** - if you have ideas on how to implement it
- **Alternatives considered** - what other solutions did you think about?

### Pull Requests

#### Process

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following our coding standards
3. **Add tests** if you've added code that should be tested
4. **Update documentation** if you've changed APIs or added features
5. **Ensure the test suite passes** (`cargo test --all`)
6. **Run clippy** (`cargo clippy -- -D warnings`)
7. **Format your code** (`cargo fmt`)
8. **Write a good commit message** following our guidelines
9. **Submit the pull request**

#### Branch Naming

- `feature/description` - for new features
- `fix/description` - for bug fixes
- `docs/description` - for documentation changes
- `refactor/description` - for refactoring
- `test/description` - for test improvements

Examples:
- `feature/ml-prediction`
- `fix/process-freeze-deadlock`
- `docs/installation-guide`

#### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Code style changes (formatting, missing semicolons, etc.)
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks, dependency updates, etc.
- `ci`: Changes to CI configuration

**Examples:**
```
feat(core): add thermal monitoring support

Implements CPU temperature tracking via /sys/class/thermal.
Allows freezing processes when temperature exceeds threshold.

Closes #42
```

```
fix(daemon): prevent deadlock in process scanner

The scanner could deadlock when multiple processes were frozen
simultaneously. Added proper mutex ordering to prevent this.

Fixes #78
```

#### Code Review Process

1. At least one maintainer must approve the PR
2. All CI checks must pass
3. Code coverage should not decrease (ideally increase)
4. No merge conflicts with `main`
5. All review comments should be addressed

## Development Setup

### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development dependencies
sudo apt-get install build-essential pkg-config libssl-dev
```

### Building from Source

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/freezr.git
cd freezr

# Create a feature branch
git checkout -b feature/my-awesome-feature

# Build the project
cargo build

# Run tests
cargo test --all

# Run clippy (linter)
cargo clippy -- -D warnings

# Format code
cargo fmt
```

### Running Locally

```bash
# Run daemon in foreground with debug logging
RUST_LOG=debug cargo run --bin freezr-daemon -- --config config/examples/development.toml

# Run CLI
cargo run --bin freezr-cli -- status

# Run specific tests
cargo test --package freezr-core --lib process_scanner

# Run integration tests
cargo test --test integration
```

### Project Structure

```
freezr/
├── crates/
│   ├── freezr-core/          # Core library
│   │   ├── src/
│   │   │   ├── lib.rs        # Public API
│   │   │   ├── scanner.rs    # Process scanning
│   │   │   ├── engine.rs     # Decision engine
│   │   │   ├── executor.rs   # Action execution
│   │   │   └── config.rs     # Configuration handling
│   │   └── tests/            # Unit tests
│   │
│   ├── freezr-daemon/        # Daemon service
│   │   ├── src/
│   │   │   ├── main.rs       # Entry point
│   │   │   ├── service.rs    # Systemd integration
│   │   │   └── monitor.rs    # Main monitoring loop
│   │   └── tests/
│   │
│   ├── freezr-cli/           # CLI tool
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/     # CLI commands
│   │   └── tests/
│   │
│   └── freezr-gui/           # GUI application (future)
│       └── src/
│
├── docs/                     # Documentation
│   ├── architecture/         # Design documents
│   ├── user-guide/          # End-user guides
│   └── api/                 # API documentation
│
├── config/
│   └── examples/            # Example configurations
│
└── tests/                   # Integration tests
    └── integration/
```

## Coding Standards

### Rust Style Guide

We follow the official [Rust Style Guide](https://rust-lang.github.io/api-guidelines/).

**Key points:**
- Use `rustfmt` for formatting (run `cargo fmt`)
- Use `clippy` for linting (run `cargo clippy`)
- All public APIs must have documentation comments (`///`)
- Use meaningful variable and function names
- Prefer explicit types when it improves readability
- Handle errors properly (no `unwrap()` in production code)

### Error Handling

```rust
// ❌ Bad - unwrap in library code
pub fn get_process_cpu(pid: u32) -> f64 {
    read_proc_stat(pid).unwrap().cpu_percent
}

// ✅ Good - return Result
pub fn get_process_cpu(pid: u32) -> Result<f64, ProcessError> {
    let stat = read_proc_stat(pid)?;
    Ok(stat.cpu_percent)
}

// ✅ Good - use custom error types
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Process {0} not found")]
    NotFound(u32),
    
    #[error("Failed to read /proc/{0}/stat: {1}")]
    ReadError(u32, #[source] std::io::Error),
}
```

### Documentation

All public APIs must be documented:

```rust
/// Scans all processes and returns their current resource usage.
///
/// This function reads `/proc/[pid]/stat` for all running processes
/// and calculates their CPU and memory usage.
///
/// # Errors
///
/// Returns `ScanError` if:
/// - Cannot read `/proc` directory
/// - Permission denied for specific process
///
/// # Examples
///
/// ```
/// use freezr_core::ProcessScanner;
///
/// let scanner = ProcessScanner::new();
/// let processes = scanner.scan_all()?;
/// for proc in processes {
///     println!("{}: {}% CPU", proc.name, proc.cpu_percent);
/// }
/// ```
pub fn scan_all(&self) -> Result<Vec<ProcessInfo>, ScanError> {
    // Implementation
}
```

### Testing

All new code should include tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_cpu_calculation() {
        let stat = ProcStat {
            utime: 100,
            stime: 50,
            /* ... */
        };
        
        let cpu = calculate_cpu_percent(&stat);
        assert!(cpu >= 0.0 && cpu <= 100.0);
    }

    #[tokio::test]
    async fn test_freeze_process() {
        let executor = ActionExecutor::new();
        
        // Create a test process
        let pid = start_test_process();
        
        // Freeze it
        executor.freeze_process(pid).await.unwrap();
        
        // Verify it's frozen
        assert!(is_process_stopped(pid));
        
        // Cleanup
        executor.unfreeze_process(pid).await.unwrap();
    }
}
```

### Performance Considerations

- **Avoid allocations in hot paths** - reuse buffers when possible
- **Use `async` for I/O operations** - but not for CPU-bound work
- **Profile before optimizing** - use `cargo bench` and `perf`
- **Document complexity** - if O(n²), explain why it's acceptable

```rust
// ✅ Good - reuse buffer
pub struct ProcessScanner {
    buffer: Vec<ProcessInfo>,
}

impl ProcessScanner {
    pub fn scan_all(&mut self) -> Result<&[ProcessInfo]> {
        self.buffer.clear();
        // Fill buffer without reallocating
        Ok(&self.buffer)
    }
}
```

## Testing

### Running Tests

```bash
# All tests
cargo test --all

# Specific package
cargo test --package freezr-core

# Specific test
cargo test test_process_scanner

# With logging output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'

# Ignored tests (long-running)
cargo test -- --ignored
```

### Test Coverage

We aim for >80% code coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage/

# Open report
xdg-open coverage/index.html
```

## Documentation

### Building Documentation

```bash
# Build API docs
cargo doc --no-deps --open

# Build mdBook (user guide)
cd docs && mdbook serve --open
```

### Documentation Standards

- All public items must have doc comments
- Include examples where appropriate
- Link to related items with `[`brackets`]`
- Use `# Safety` section for `unsafe` code
- Use `# Panics` section if function can panic
- Use `# Errors` section for `Result` returns

## Release Process

(For maintainers)

1. Update version in all `Cargo.toml` files
2. Update `CHANGELOG.md`
3. Create git tag: `git tag -a v0.2.0 -m "Release v0.2.0"`
4. Push tag: `git push origin v0.2.0`
5. GitHub Actions will automatically:
   - Build binaries for all platforms
   - Create GitHub release
   - Publish to crates.io

## Questions?

Don't hesitate to ask! You can:
- Open a [Discussion](https://github.com/YOUR_USERNAME/freezr/discussions)
- Ask in existing Issues
- Contact maintainers directly

## License

By contributing to FreezR, you agree that your contributions will be dual-licensed under MIT and Apache-2.0.
