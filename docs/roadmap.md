# Roadmap

This roadmap prioritizes developer experience while delivering advanced NUMA control. Each phase ends with a usable, documented, and testable slice of functionality.

**Guiding goals**
- Make advanced NUMA easy to adopt in real systems.
- Fail fast and explain why when a hard-mode invariant cannot be enforced.
- Keep ergonomics high even when exposing low-level controls.

**Phase 0: Repository foundation**
Outcomes:
- Workspace layout with crate skeletons.
- CI that builds on Linux and runs unit tests.
- Shared error and capability model.

DX focus:
- Consistent error types with actionable messages.
- `numaperf-core` crate for shared types and configuration.
- A single config entry point for soft vs hard mode.

**Phase 1: Topology and affinity MVP**
Outcomes:
- `numaperf-topo` discovers NUMA nodes, sockets, cores, SMT, and LLC groups.
- `numaperf-affinity` pins the current thread and spawns pinned workers.

DX focus:
- Stable identifiers and CPU set helpers.
- Debug output that prints the topology in a human-readable form.
- Example: "pin workers by node".

Exit criteria:
- Deterministic topology discovery on test machines.
- Affinity ops are unit-tested and safely reversible.

**Phase 2: Memory placement MVP**
Outcomes:
- `numaperf-mem` supports `Bind`, `Preferred`, `Interleave`, and `Local` policies.
- Anonymous region allocation with optional prefault.
- A minimal file-backed mapping path with explicit policy.

DX focus:
- Clear policy semantics in docs and API docs.
- Prefault helpers with safe defaults.
- Errors that explain missing kernel support or privileges.

Exit criteria:
- Integration tests show policy application and prefault behavior.

**Phase 3: Scheduler MVP**
Outcomes:
- `numaperf-sched` with per-node queues.
- Configurable steal order: local, socket, remote.
- Basic job tagging with home-node affinity.

DX focus:
- Zero-configuration mode that still respects locality.
- Optional explicit configuration for strict placement.
- Example: "NUMA-aware worker pool".

Exit criteria:
- Deterministic worker placement and queue locality under load.

**Phase 4: Sharded structures and IO locality**
Outcomes:
- `numaperf-sharded` with per-node shards, counters, and cache padding.
- `numaperf-io` with device-to-node mapping and IO worker pools.

DX focus:
- Simple constructors that infer node count from topology.
- Lightweight ergonomics for read-mostly global views.

Exit criteria:
- Bench tests show reduced contention and consistent locality.

**Phase 5: Observability and diagnostics**
Outcomes:
- `numaperf-perf` with per-node allocation and queue stats.
- A diagnostic report API that summarizes locality health.

DX focus:
- A single `LocalityReport` type with human-readable summaries.
- An optional "advanced mode" behind a feature flag for perf counters.

Exit criteria:
- The diagnostics API can explain remote traffic spikes.

**Phase 6: Hard mode enforcement**
Outcomes:
- Strict enforcement of pinning and memory policies.
- Structured errors when hard-mode invariants cannot be enforced.
- A capability matrix that surfaces required kernel features.

DX focus:
- A "hard-mode checklist" in docs.
- Explicit fallbacks for soft mode with warnings.

Exit criteria:
- Hard-mode failure cases are deterministic and well documented.

**Phase 7: Stability and ecosystem readiness**
Outcomes:
- Public API stabilization for core crates.
- Versioned compatibility policy.
- Comprehensive docs and examples.

DX focus:
- "Getting Started" guide with minimal boilerplate.
- Integration examples with a simple executor and a buffer pool.
- Clear upgrade notes and migration guide.

Exit criteria:
- Adoption by at least one real system without internal patches.

**Benchmark plan (assume NUMA hardware)**
Benchmarks validate that `numaperf` improves locality and avoids regressions. All benchmarks assume a multi-socket NUMA machine and include a single-socket fallback mode for smoke checks only.

Bench categories:
- Topology and affinity: pinning overhead, migration rate, and CPU set correctness under load.
- Memory placement: allocation latency, local vs remote bandwidth, and prefault time.
- Scheduling: local queue hit rate, cross-node steal rate, and throughput scaling.
- Sharded structures: contention reduction and throughput vs a global lock baseline.
- Observability: overhead of counters, snapshot latency, and report clarity.

Required benchmark features:
- A `numaperf-bench` binary with a unified CLI and consistent output format.
- A “locality health” summary per run, including remote access ratios.
- Configurable policies for `Bind`, `Preferred`, `Interleave`, and `Local`.
- JSON output for CI trend tracking and regression detection.

Success thresholds:
- Pinning overhead stays below a fixed per-thread budget.
- Local bandwidth is within 90% of the machine’s measured maximum.
- Remote access ratios fall when hard mode is enabled vs soft mode.
- Scheduler throughput scales with additional nodes without collapsing locality.

Exit criteria:
- Bench results are reproducible across at least two different NUMA machines.
- Benchmark scripts and expected outputs are documented and versioned.

**Cross-cutting developer experience investments**
- Unified configuration model across all crates.
- Consistent logging and metrics naming.
- Doc site layout with a stable URL structure.
- Examples that can be run on a single-socket machine.
- Capability detection APIs that return precise reasons for missing features.

**Docs (current)**
- `docs/architecture.md`
- `docs/api.md`
- `docs/roadmap.md`
- `docs/kernel-requirements.md`

**Docs (planned)**
- `docs/getting-started.md`
- `docs/hard-mode.md`
