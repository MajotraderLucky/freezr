# FreezR ML Process Analytics System 🤖

## Концепция

Интеллектуальная система сбора, анализа и оптимизации процессов на основе машинного обучения.

**Цель**: Автоматически изучать поведение процессов, выявлять паттерны и предлагать оптимизации.

---

## 🏗️ Архитектура системы

```
┌─────────────────────────────────────────────────────────────┐
│                    FreezR Process Monitor                   │
│                  (Real-time monitoring)                      │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│              Statistics Collector (NEW)                      │
│  • CPU/Memory time-series data                              │
│  • Process lifecycle events                                 │
│  • Resource usage patterns                                  │
│  • Context switches, I/O stats                              │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                Time-Series Database                          │
│  Format: Parquet/CSV (для ML) + SQLite (для queries)       │
│  • Process snapshots (every 3-5 seconds)                    │
│  • Aggregated metrics (hourly, daily)                       │
│  • Event logs (kills, freezes, restarts)                    │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│              Pattern Analyzer (ML Pipeline)                  │
│  • Feature engineering                                       │
│  • Anomaly detection                                         │
│  • Clustering (group similar processes)                     │
│  • Time-series forecasting (predict spikes)                 │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│            Recommendation Engine                             │
│  • Process optimization suggestions                          │
│  • Configuration tuning                                      │
│  • Proactive actions                                         │
└─────────────────────────────────────────────────────────────┘
```

---

## 📊 Data Collection Strategy

### 1. Process Snapshot (каждые 3-5 секунд)

**Собираемые метрики:**

```rust
struct ProcessSnapshot {
    // Identity
    pid: u32,
    name: String,
    cmdline: String,
    user: String,

    // Timestamps
    timestamp: DateTime<Utc>,
    start_time: DateTime<Utc>,
    uptime_seconds: u64,

    // Resource Usage
    cpu_percent: f64,
    memory_rss_mb: u64,
    memory_vms_mb: u64,
    memory_percent: f64,

    // I/O Statistics (from /proc/[pid]/io)
    read_bytes: u64,
    write_bytes: u64,
    read_ops: u64,
    write_ops: u64,

    // CPU Details
    user_time: u64,      // CPU time in user mode
    system_time: u64,    // CPU time in kernel mode
    num_threads: u32,

    // Context & Priority
    voluntary_ctxt_switches: u64,
    nonvoluntary_ctxt_switches: u64,
    nice_value: i32,
    priority: i32,

    // State
    state: ProcessState,  // Running, Sleeping, Zombie, etc.

    // Classification
    category: ProcessCategory, // Browser, IDE, Build, System, etc.
}

enum ProcessCategory {
    Browser,
    IDE,
    BuildTool,
    SystemService,
    Antivirus,
    Background,
    Unknown,
}
```

### 2. Event Logging

**Критичные события:**

```rust
struct ProcessEvent {
    timestamp: DateTime<Utc>,
    pid: u32,
    process_name: String,
    event_type: EventType,
    details: EventDetails,
}

enum EventType {
    // Lifecycle
    ProcessStarted,
    ProcessExited { exit_code: i32 },
    ProcessKilled { signal: i32 },

    // Actions
    ProcessFrozen { duration_sec: u64 },
    ProcessUnfrozen,
    ServiceRestarted,
    NiceAdjusted { old_nice: i32, new_nice: i32 },

    // Violations
    CpuViolation { cpu_percent: f64, threshold: f64 },
    MemoryViolation { memory_mb: u64, threshold: u64 },

    // Anomalies
    AnomalyDetected { anomaly_score: f64, description: String },
    UnusualBehavior { reason: String },
}
```

### 3. Aggregated Metrics (hourly/daily)

**Для долгосрочного анализа:**

```rust
struct ProcessDailySummary {
    date: NaiveDate,
    process_name: String,

    // Runtime
    total_runtime_seconds: u64,
    num_starts: u32,
    num_kills: u32,
    num_crashes: u32,

    // Resource Stats
    avg_cpu_percent: f64,
    max_cpu_percent: f64,
    avg_memory_mb: u64,
    max_memory_mb: u64,

    // I/O Stats
    total_read_gb: f64,
    total_write_gb: f64,

    // Violations
    cpu_violations: u32,
    memory_violations: u32,

    // Behavior
    typical_runtime_hours: Vec<u8>, // [0-23] когда обычно запускается
    avg_uptime_minutes: u64,
}
```

---

## 💾 Data Storage Design

### Storage Format

**Primary Storage: Parquet files** (for ML)
- Columnar format, отличная компрессия
- Быстрое чтение для pandas/polars
- Partitioning по дате: `data/process_stats/year=2025/month=10/day=26/snapshots.parquet`

**Secondary Storage: SQLite** (for queries)
- Быстрые ad-hoc queries
- Aggregated metrics
- Event logs

### Directory Structure

```
freezr/
└── data/
    ├── process_stats/
    │   ├── snapshots/         # Raw snapshots (Parquet)
    │   │   ├── 2025-10-26.parquet
    │   │   ├── 2025-10-27.parquet
    │   │   └── ...
    │   ├── events/            # Event logs (Parquet)
    │   │   ├── 2025-10-26.parquet
    │   │   └── ...
    │   └── aggregated/        # Daily summaries (Parquet)
    │       ├── daily_2025-10.parquet
    │       └── ...
    ├── analytics.db           # SQLite для queries
    └── ml/
        ├── datasets/          # Prepared ML datasets
        ├── models/            # Trained models
        └── reports/           # Analysis reports
```

---

## 🤖 ML Pipeline

### Phase 1: Pattern Recognition (Unsupervised Learning)

#### 1.1 Process Clustering

**Goal**: Группировка похожих процессов по поведению

**Algorithm**: K-Means или DBSCAN

**Features**:
- Average CPU usage
- Average memory usage
- I/O intensity (read/write ratio)
- Uptime distribution
- Time of day patterns

**Output**: Process profiles
- "Heavy CPU users" (builds, encoders)
- "Memory hogs" (browsers, IDEs)
- "I/O intensive" (databases, file indexers)
- "Background services" (low resource, always running)

#### 1.2 Anomaly Detection

**Goal**: Выявление необычного поведения процессов

**Algorithm**: Isolation Forest или Autoencoder

**Features**:
- CPU deviation from baseline
- Memory growth rate
- Unexpected I/O spikes
- Context switch anomalies

**Use Cases**:
- Malware detection (unusual CPU patterns)
- Memory leaks (steady memory growth)
- Runaway processes (sudden spike)

### Phase 2: Time-Series Forecasting (Supervised Learning)

#### 2.1 CPU Spike Prediction

**Goal**: Предсказать CPU spike за 5-30 секунд

**Algorithm**: LSTM или Transformer

**Input**:
- Last 30 seconds of CPU usage (time window)
- Process name embedding
- Time of day features
- System load context

**Output**: Probability of CPU spike in next 30s

**Action**: Preemptive nice или cgroup limit

#### 2.2 Memory Growth Prediction

**Goal**: Предсказать memory leak или OOM

**Algorithm**: Linear regression + anomaly detection

**Features**:
- Memory usage trend (slope)
- Process uptime
- Historical memory patterns

**Output**: Estimated time to OOM

### Phase 3: Recommendation Engine

#### 3.1 Process Optimization Suggestions

**Based on collected data, suggest:**

1. **Nice value adjustments**
   ```
   Process: build-server
   Observation: Runs only during working hours, high CPU but not interactive
   Suggestion: Set nice=10 to reduce priority
   Expected benefit: 15% better system responsiveness
   ```

2. **Cgroup limits**
   ```
   Process: kesl
   Observation: Occasionally spikes to 100% CPU for 5+ seconds
   Suggestion: Set cgroup CPU quota to 30%
   Expected benefit: Prevent system freezes
   ```

3. **Process scheduling**
   ```
   Process: backup-daemon
   Observation: High I/O usage during working hours
   Suggestion: Reschedule to run at night (cron: 0 2 * * *)
   Expected benefit: No I/O contention during work
   ```

4. **Memory optimization**
   ```
   Process: chrome
   Observation: Memory grows to 4GB+ after 8 hours uptime
   Suggestion: Restart chrome every 6 hours (or use tab suspender)
   Expected benefit: Prevent OOM
   ```

---

## 🛠️ Implementation Plan

### Step 1: Data Collection (Week 1)

#### 1.1 Extend ProcessMonitor
- [x] Already have basic CPU/memory monitoring
- [ ] Add I/O statistics collection
- [ ] Add context switch tracking
- [ ] Add process categorization

#### 1.2 Implement ProcessSnapshot storage
- [ ] Create Parquet writer (use `arrow` or `polars` crate)
- [ ] Implement buffered writes (collect 1000 snapshots → write batch)
- [ ] Add daily file rotation
- [ ] Implement compression (snappy or zstd)

#### 1.3 Create Event Logger
- [ ] Log all ProcessEvents to Parquet
- [ ] Include action outcomes (success/failure)
- [ ] Track correlation (event → action → outcome)

### Step 2: Data Analysis Tools (Week 2)

#### 2.1 Dataset Generator
```bash
# Generate ML-ready dataset
freezr-cli analyze generate-dataset \
  --start-date 2025-10-01 \
  --end-date 2025-10-26 \
  --output ml_dataset.parquet
```

#### 2.2 Statistics Dashboard
```bash
# Show insights from collected data
freezr-cli analyze summary \
  --process firefox \
  --last 7d
```

**Output**:
```
Firefox Statistics (Last 7 days)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Runtime: 45.2 hours total
📈 Avg CPU: 12.3% (max: 187.4%)
💾 Avg Memory: 2.1GB (max: 4.8GB)
🔥 CPU spikes: 23 events >80%
⚠️  Violations: 5 kills triggered
📉 Trend: Memory grows 50MB/hour
💡 Recommendation: Restart every 8 hours
```

### Step 3: ML Model Training (Week 3-4)

#### 3.1 Python ML Pipeline

**Technology Stack**:
- **Data loading**: Polars (faster than pandas)
- **ML framework**: scikit-learn + PyTorch/TensorFlow
- **Feature engineering**: Custom Rust → Python bridge
- **Model serving**: ONNX runtime (run models from Rust)

**Training script**:
```python
# ml/train_cpu_predictor.py
import polars as pl
from sklearn.ensemble import IsolationForest
import torch

# Load data
df = pl.read_parquet("data/process_stats/snapshots/*.parquet")

# Feature engineering
features = engineer_features(df)

# Train anomaly detector
model = IsolationForest(contamination=0.01)
model.fit(features)

# Save model
model.save("data/ml/models/anomaly_detector.pkl")
```

#### 3.2 LSTM for CPU Spike Prediction

```python
# ml/train_lstm.py
class CPUSpikePredictor(nn.Module):
    def __init__(self, input_size=10, hidden_size=64):
        super().__init__()
        self.lstm = nn.LSTM(input_size, hidden_size, num_layers=2)
        self.fc = nn.Linear(hidden_size, 1)
        self.sigmoid = nn.Sigmoid()

    def forward(self, x):
        # x shape: (seq_len, batch, features)
        lstm_out, _ = self.lstm(x)
        prediction = self.sigmoid(self.fc(lstm_out[-1]))
        return prediction  # Probability of spike

# Train on historical data
model = CPUSpikePredictor()
train_model(model, train_loader)

# Export to ONNX for Rust
torch.onnx.export(model, dummy_input, "cpu_spike_model.onnx")
```

### Step 4: Integration with FreezR (Week 5)

#### 4.1 Load ML Models in Rust

```rust
use tract_onnx::prelude::*;

struct MLPredictor {
    spike_model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
}

impl MLPredictor {
    pub fn new() -> Result<Self> {
        let model = tract_onnx::onnx()
            .model_for_path("data/ml/models/cpu_spike_model.onnx")?
            .into_optimized()?
            .into_runnable()?;

        Ok(Self { spike_model: model })
    }

    pub fn predict_spike(&self, cpu_history: &[f64]) -> Result<f64> {
        // Prepare input tensor
        let input = tract_ndarray::arr1(cpu_history).into_shape((30, 1, 1))?;

        // Run inference
        let result = self.spike_model.run(tvec!(input.into()))?;

        // Extract probability
        let probability = result[0].to_array_view::<f32>()?.iter().next().unwrap();
        Ok(*probability as f64)
    }
}
```

#### 4.2 Proactive Actions

```rust
// In monitoring loop
if let Some(spike_prob) = ml_predictor.predict_spike(&cpu_history) {
    if spike_prob > 0.7 {
        info!("ML predicts CPU spike ({}% probability), taking proactive action",
              spike_prob * 100.0);

        // Preemptive nice adjustment
        process.set_nice(10)?;
    }
}
```

---

## 📈 Expected Benefits

### Short-term (1 month of data)
- ✅ Identify resource-heavy processes
- ✅ Detect unusual patterns (potential malware)
- ✅ Basic recommendations (nice values, limits)

### Medium-term (3-6 months of data)
- ✅ Accurate CPU spike predictions (5-10s ahead)
- ✅ Memory leak detection
- ✅ Automated optimization suggestions
- ✅ Process behavior profiles

### Long-term (1+ year of data)
- ✅ System-wide optimization
- ✅ Predictive maintenance (know when to upgrade RAM)
- ✅ Workload pattern analysis (weekday vs weekend)
- ✅ Energy usage optimization

---

## 🎯 Success Metrics

### Data Collection
- **Coverage**: >95% uptime for monitoring
- **Storage**: <100MB/day for raw data
- **Compression**: >70% with Parquet

### ML Models
- **CPU spike prediction**: >80% accuracy, <5% false positives
- **Anomaly detection**: Detect >90% of unusual behavior
- **Inference time**: <10ms per prediction

### Recommendations
- **Acceptance rate**: >50% of suggestions applied by user
- **Effectiveness**: >30% reduction in violations after applying suggestions

---

## 🚀 Quick Start Commands (Future)

```bash
# Start collecting statistics
freezr-daemon --enable-ml-collection

# Generate weekly report
freezr-cli analyze report --last 7d

# Train models on collected data
freezr-cli ml train --all

# Get optimization suggestions
freezr-cli ml suggest

# Export dataset for external analysis
freezr-cli analyze export --format parquet --output my_dataset.parquet
```

---

## 📚 Related Documentation

- [ROADMAP.md](ROADMAP.md) - Overall project roadmap
- [LOG_MAINTENANCE.md](LOG_MAINTENANCE.md) - Log management system
- [PROCESS_MONITOR_SUMMARY.md](PROCESS_MONITOR_SUMMARY.md) - Current monitoring capabilities

---

**Status**: Design Phase
**Next Step**: Implement ProcessSnapshot collection
**Expected Timeline**: 5-6 weeks to MVP
