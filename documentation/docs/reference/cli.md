# CLI Tools

Reference for the `numaperf-bench` command-line tool.

## Installation

```bash
cargo install numaperf-bench

# Or build from source
cargo build --release -p numaperf-bench
```

## Commands

### numaperf-bench

Main entry point for NUMA benchmarking and system information.

```bash
numaperf-bench [COMMAND]
```

If no command is specified, defaults to `info`.

---

## info

Display system topology, capabilities, and current state.

```bash
numaperf-bench info [SECTION] [OPTIONS]
```

### Sections

| Section | Description |
|---------|-------------|
| `all` | Show all information (default) |
| `topology` | NUMA topology (nodes, CPUs, memory) |
| `capabilities` | System capabilities for hard mode |
| `affinity` | Current thread CPU affinity |
| `distances` | NUMA node distance matrix |

### Options

| Option | Description |
|--------|-------------|
| `-f, --format <FORMAT>` | Output format: `text` or `json` (default: text) |
| `-v, --verbose` | Verbose output with recommendations |

### Examples

```bash
# Show all system info
numaperf-bench info

# Show only topology
numaperf-bench info topology

# Show capabilities in JSON format
numaperf-bench info capabilities --format json

# Verbose output with recommendations
numaperf-bench info --verbose
```

### Sample Output

```
=== numaperf System Information ===

Topology
--------
NUMA Nodes: 2

  Node 0:
    CPUs: 0-7 (8 total)

  Node 1:
    CPUs: 8-15 (8 total)

Capabilities
------------
Hard mode supported: yes
  Strict memory binding: yes
  Strict CPU affinity: yes
  Memory locking: yes
  NUMA balancing disabled: yes

Current Affinity
----------------
Allowed CPUs: 0-15 (16 total)
Currently on CPU: 5

Distance Matrix
---------------
       Node 0  Node 1
Node 0     10      20
Node 1     20      10
```

---

## bench

Run NUMA locality and performance benchmarks.

```bash
numaperf-bench bench [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `-c, --category <CATEGORY>` | Benchmark category (default: all) |
| `-f, --format <FORMAT>` | Output format: `text` or `json` (default: text) |
| `-i, --iterations <N>` | Number of iterations (default: 1000) |
| `-t, --threads <N>` | Number of threads (default: all CPUs) |

### Categories

| Category | Description |
|----------|-------------|
| `all` | Run all benchmarks |
| `sharded` | Per-node sharded data structures |
| `memory` | NUMA-aware memory allocation |
| `scheduler` | Executor and work distribution |
| `affinity` | Thread pinning operations |

### Examples

```bash
# Run all benchmarks
numaperf-bench bench

# Run only memory benchmarks
numaperf-bench bench --category memory

# JSON output for CI/CD
numaperf-bench bench --format json

# Custom iterations
numaperf-bench bench --iterations 10000

# Limit thread count
numaperf-bench bench --threads 8
```

### Sample Output

```
=== numaperf Benchmark Suite ===

System: 2 NUMA nodes, 16 CPUs
Hard mode supported: true

Running sharded benchmarks...
  sharded_counter_increment     : 45,678,901 ops/sec
  sharded_counter_sum           :  5,432,109 ops/sec
  numa_sharded_local_access     : 23,456,789 ops/sec

Running memory benchmarks...
  numa_region_alloc_1mb         :      1,234 ops/sec
  numa_region_alloc_bind        :      1,198 ops/sec
  numa_region_write_throughput  :  8,765,432 MB/sec

Running scheduler benchmarks...
  executor_submit_local         :  1,234,567 ops/sec
  executor_submit_remote        :    987,654 ops/sec (locality: 55.6%)
  work_distribution             :  2,345,678 ops/sec (locality: 92.3%)

Running affinity benchmarks...
  scoped_pin_create             :    345,678 ops/sec
  get_affinity                  :  5,678,901 ops/sec

=== Summary ===
Overall locality ratio: 89.2%
Health: Good

Recommendations:
  - Consider using LocalOnly steal policy for latency-sensitive workloads
```

### JSON Output Format

```json
{
  "system": {
    "numa_nodes": 2,
    "cpus": 16,
    "hard_mode_supported": true
  },
  "benchmarks": [
    {
      "name": "sharded_counter_increment",
      "ops_per_sec": 45678901,
      "duration_ns": 21893,
      "operations": 1000,
      "locality_ratio": null
    },
    {
      "name": "work_distribution",
      "ops_per_sec": 2345678,
      "duration_ns": 426331,
      "operations": 1000,
      "locality_ratio": 0.923
    }
  ],
  "locality_health": {
    "ratio": 0.892,
    "status": "good"
  }
}
```

---

## Criterion Benchmarks

For detailed benchmarking with statistical analysis, use Criterion:

```bash
# Run all Criterion benchmarks
cargo bench -p numaperf-bench

# Run specific benchmark group
cargo bench -p numaperf-bench -- sharded
cargo bench -p numaperf-bench -- memory
cargo bench -p numaperf-bench -- scheduler
cargo bench -p numaperf-bench -- affinity

# Generate HTML report
cargo bench -p numaperf-bench -- --plotting-backend gnuplot
```

Criterion reports are saved to `target/criterion/`.

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Runtime error (topology discovery failed, etc.) |
| 2 | Invalid arguments |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level (e.g., `numaperf=debug`) |
| `NO_COLOR` | Disable colored output |

---

## See Also

- [Basic Topology Example](../examples/basic-topology.md)
- [Performance Tuning](../advanced/performance-tuning.md)
