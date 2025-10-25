# FreezR Usage Examples

Real-world scenarios and configuration examples.

## Available Examples

### [Common Scenarios](common-scenarios.md)
10 detailed scenarios covering:

1. **Monitoring KESL Service** - Focus on KESL antivirus monitoring
2. **Monitoring Node.js Processes** - Detect and kill hung Node processes
3. **Combined KESL + Node** - Monitor both with different thresholds
4. **Conservative Monitoring** - Testing without aggressive actions
5. **Aggressive Protection** - Production server setup
6. **Development Environment** - Dev machine with Node.js tools
7. **Scheduled Monitoring** - Periodic checks via cron
8. **Multi-Instance Deployment** - Different policies for different services
9. **Debugging** - Detailed logging for troubleshooting
10. **Dry-Run Mode** - Simulation without actions

Each scenario includes:
- Problem description
- Complete configuration
- Run commands
- Expected behavior
- Log output examples

## Example Configurations

All examples include full TOML configuration files that can be copied and customized.

## Quick Reference

| Scenario | Key Features | Best For |
|----------|-------------|----------|
| KESL Only | Focus on antivirus | Production servers with KESL |
| Node Only | Node.js monitoring | Development machines |
| Conservative | High thresholds | Testing and tuning |
| Aggressive | Low thresholds | Critical production servers |
| Development | Balanced monitoring | Developer workstations |
| Multi-Instance | Separate configs | Complex deployments |

## Quick Links

- [View All Scenarios →](common-scenarios.md)
- [Best Practices →](common-scenarios.md#best-practices)
- [← Back to Main Docs](../README.md)
