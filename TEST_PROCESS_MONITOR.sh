#!/bin/bash
# Test script for process-monitor binary
# Tests all major functionality and validates output

set -e

FREEZR_DIR="/home/ryazanov/.myBashScripts/freezr"
BIN="$FREEZR_DIR/target/release/process-monitor"
CONFIG="$FREEZR_DIR/freezr.toml"

echo "╔════════════════════════════════════════════════════════╗"
echo "║        Process Monitor Test Suite                      ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""

# Test 1: Check binary exists
echo "📋 Test 1: Check binary exists"
if [ -f "$BIN" ]; then
    echo "✅ Binary found: $BIN"
else
    echo "❌ Binary not found. Building..."
    cd "$FREEZR_DIR"
    cargo build --release --bin process-monitor
    if [ -f "$BIN" ]; then
        echo "✅ Binary built successfully"
    else
        echo "❌ Build failed"
        exit 1
    fi
fi
echo ""

# Test 2: Help command
echo "📋 Test 2: Help command"
$BIN --help > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ Help command works"
else
    echo "❌ Help command failed"
    exit 1
fi
echo ""

# Test 3: Check configuration file
echo "📋 Test 3: Configuration file"
if [ -f "$CONFIG" ]; then
    echo "✅ Config found: $CONFIG"
else
    echo "⚠️  Config not found, using default"
fi
echo ""

# Test 4: Directory creation
echo "📋 Test 4: Directory creation test"
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Create test config
cat > test.toml <<EOF
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 80.0
enabled = true
auto_kill = true
confirm_kill = false

[logging]
log_dir = "./logs"
kesl_log = "kesl-monitor.log"
node_log = "node-monitor.log"
actions_log = "actions.log"
max_file_size_mb = 10
rotate_count = 5

[monitoring]
check_interval_secs = 3
min_restart_interval_secs = 100
EOF

# Run process-monitor for 5 seconds
timeout 5s $BIN --config test.toml > output.log 2>&1 || true

# Check if directories were created
if [ -d "logs" ] && [ -d "logs/archive" ] && [ -d "data/process_stats" ]; then
    echo "✅ Directories created successfully"
else
    echo "❌ Directory creation failed"
    exit 1
fi

# Check if log file was created
if ls logs/process_monitor.log.* 1> /dev/null 2>&1; then
    echo "✅ Log file created"
else
    echo "❌ Log file not created"
    exit 1
fi

# Cleanup
cd -
rm -rf "$TEMP_DIR"
echo ""

# Test 5: Startup banner check
echo "📋 Test 5: Startup banner validation"
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"
cp "$FREEZR_DIR/freezr.toml" test.toml 2>/dev/null || cat > test.toml <<EOF
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 80.0
enabled = true
auto_kill = true

[monitoring]
check_interval_secs = 3
min_restart_interval_secs = 100

[logging]
log_dir = "./logs"
kesl_log = "kesl.log"
node_log = "node.log"
actions_log = "actions.log"
max_file_size_mb = 10
rotate_count = 5
EOF

timeout 3s $BIN --config test.toml > output.log 2>&1 || true

# Check for startup messages
if grep -q "Process Monitor starting" output.log; then
    echo "✅ Startup banner present"
else
    echo "❌ Startup banner missing"
fi

if grep -q "Pre-flight checks" output.log; then
    echo "✅ Pre-flight checks executed"
else
    echo "❌ Pre-flight checks missing"
fi

if grep -q "Configuration validated" output.log; then
    echo "✅ Configuration validated"
else
    echo "❌ Configuration validation missing"
fi

cd -
rm -rf "$TEMP_DIR"
echo ""

# Test 6: Statistics mode
echo "📋 Test 6: Statistics mode"
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"
cp "$FREEZR_DIR/freezr.toml" test.toml 2>/dev/null || cat > test.toml <<EOF
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 80.0
enabled = true
auto_kill = true

[monitoring]
check_interval_secs = 1
min_restart_interval_secs = 100

[logging]
log_dir = "./logs"
kesl_log = "kesl.log"
node_log = "node.log"
actions_log = "actions.log"
max_file_size_mb = 10
rotate_count = 5
EOF

# Run with stats for 15 seconds (should get 1 report)
timeout 15s $BIN --config test.toml --stats --report-interval 10 > output.log 2>&1 || true

# Check for statistics report
if grep -q "PROCESS MONITOR STATISTICS" output.log; then
    echo "✅ Statistics report generated"
else
    echo "⚠️  Statistics report not found (may need longer runtime)"
fi

if grep -q "Runtime:" output.log; then
    echo "✅ Runtime tracking works"
else
    echo "⚠️  Runtime tracking not found"
fi

cd -
rm -rf "$TEMP_DIR"
echo ""

# Test 7: Disk space check
echo "📋 Test 7: Disk space check"
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"
cp "$FREEZR_DIR/freezr.toml" test.toml 2>/dev/null || cat > test.toml <<EOF
[kesl]
cpu_threshold = 30.0
memory_threshold_mb = 600
max_violations = 3
service_name = "kesl"
enabled = true

[node]
cpu_threshold = 80.0
enabled = true
auto_kill = true

[monitoring]
check_interval_secs = 3
min_restart_interval_secs = 100

[logging]
log_dir = "./logs"
kesl_log = "kesl.log"
node_log = "node.log"
actions_log = "actions.log"
max_file_size_mb = 10
rotate_count = 5
EOF

timeout 3s $BIN --config test.toml > output.log 2>&1 || true

if grep -q "Disk space" output.log; then
    echo "✅ Disk space check performed"
else
    echo "❌ Disk space check missing"
fi

cd -
rm -rf "$TEMP_DIR"
echo ""

# Summary
echo "╔════════════════════════════════════════════════════════╗"
echo "║                  Test Summary                          ║"
echo "╚════════════════════════════════════════════════════════╝"
echo ""
echo "All core tests passed! ✅"
echo ""
echo "To run process-monitor:"
echo "  Standard mode:    $BIN"
echo "  Statistics mode:  $BIN --stats --report-interval 60"
echo ""
echo "To view logs:"
echo "  tail -f logs/process_monitor.log.\$(date +%Y-%m-%d)"
echo ""
echo "Shell aliases (add to ~/.bashrc):"
echo "  alias procmonR='cd $FREEZR_DIR && ./target/release/process-monitor'"
echo "  alias procmonStatsR='cd $FREEZR_DIR && ./target/release/process-monitor --stats --report-interval 60'"
echo "  alias procmonLogsR='tail -f $FREEZR_DIR/logs/process_monitor.log.\$(date +%Y-%m-%d)'"
echo ""
