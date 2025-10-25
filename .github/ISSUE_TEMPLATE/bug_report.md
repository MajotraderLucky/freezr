---
name: Bug report
about: Create a report to help us improve FreezR
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug Description
A clear and concise description of what the bug is.

## To Reproduce
Steps to reproduce the behavior:
1. Start FreezR with config '...'
2. Run process '...'
3. Observe behavior '...'
4. See error

## Expected Behavior
A clear and concise description of what you expected to happen.

## Actual Behavior
What actually happened instead.

## Environment
- **OS:** [e.g., Ubuntu 22.04, Arch Linux]
- **Kernel:** [e.g., 6.2.0-39-generic] (run `uname -r`)
- **FreezR version:** [e.g., 0.1.0] (run `freezr --version`)
- **Rust version:** [e.g., 1.70.0] (run `rustc --version`)
- **Installation method:** [source, cargo, AUR, .deb, etc.]

## Configuration
Please provide your `config.toml` (remove sensitive information):
```toml
# Paste your config here
```

## Logs
Please provide relevant logs. Increase log level if needed:
```bash
# Run daemon with debug logging
RUST_LOG=debug freezr-daemon

# Or check systemd logs
journalctl -u freezr -n 100
```

```
# Paste logs here
```

## Additional Context
Add any other context about the problem here.

### Screenshots
If applicable, add screenshots to help explain your problem.

### Related Issues
Are there any related issues? Link them here.

## Checklist
- [ ] I have searched existing issues to avoid duplicates
- [ ] I have provided all requested information
- [ ] I have included logs with RUST_LOG=debug
- [ ] I have tested with the latest version
