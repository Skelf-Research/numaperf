Got it — “hard mode” means we stop being polite to the OS and start **controlling placement, pinning, page faulting, huge pages, and cross-socket traffic** explicitly. Think of it as **NUMA as a first-class runtime**.

Below is a concrete design for a Rust “hard mode” library suite you can drop under a DB (and other bandwidth-bound systems). It’s Linux-first and assumes big iron.

---

## Design principles for hard mode

1. **Pin first, allocate second, touch third**

   * If you allocate before pinning, you lose first-touch locality.
2. **Explicit memory policies for “big buffers”**

   * Mimalloc is great for general allocations, but DB hot paths live in large regions (buffer pool frames, hash tables, sort scratch). Those need explicit node placement.
3. **Everything sharded by NUMA node**

   * queues, caches, hash tables, counters, allocators, even metrics.
4. **Cross-node traffic is an observable**

   * you must measure remote bandwidth/latency pressure and surface it.

---

## Crate suite (hard mode)

### 1) `neul_topo` — topology discovery (hwloc-backed)

**Goal:** authoritative mapping:

* NUMA nodes, sockets, cores, SMT threads
* LLC groups
* PCI locality (NVMe + NIC proximity)

**Backend:** `hwloc` via `hwlocality` crate (or direct bindings).
**Why:** avoids fragile `/sys` parsing and supports IO locality.

Key API:

* `Topology::discover() -> Topology`
* `topo.numa_nodes() -> &[Node]`
* `topo.cpu_set(node_id) -> CpuSet`
* `topo.closest_node_for_pci(pci_bdf) -> NodeId`

---

### 2) `neul_affinity` — strict pinning + “no migration”

**Goal:** make pinning non-optional.

Key features:

* pin current thread to a **CpuSet** (not just one core)
* optional “exclusive” sets (avoid siblings/SMT if desired)
* enforce `sched_setaffinity` + optionally `sched_setscheduler` (SCHED_FIFO for special workers if you dare)

Key API:

* `pin_current(CpuSet)`
* `spawn_pinned(node_id, || ...)`
* `ScopedPin` guard (restores old affinity)

---

### 3) `neul_mem` — NUMA memory placement (the core)

This is the heart. It provides **explicit placement** for:

* anonymous memory (malloc/mmap)
* file-backed mmap (WAL, segments)
* migration + interleave policies
* huge pages + prefault

#### Backend calls (Linux)

* `mmap`, `munmap`
* `mbind`, `set_mempolicy`, `get_mempolicy`
* `madvise` (`WILLNEED`, `HUGEPAGE`, `NOHUGEPAGE`, `DONTNEED`)
* `move_pages` for migration (optional)
* `mlock`/`mlock2` (optional)
* `perf_event_open` hooks elsewhere for observability

Use either:

* `libnuma` (straightforward) for node masks + policy calls, **plus** direct syscalls where needed, or
* hwloc memory binding APIs (works, but libnuma is often simpler for policy primitives).

#### Memory policy model

Expose a single enum that maps cleanly to Linux policies:

```rust
enum MemPolicy {
  Bind(NodeMask),        // strict local; fail or fallback based on flags
  Preferred(NodeId),     // prefer local, allow remote
  Interleave(NodeMask),  // spread pages (great for read-mostly scans)
  Local,                 // always allocate on current thread’s node
}
```

#### Allocation primitives you actually need

* `NumaRegion`: big contiguous region with placement policy
* `NumaMmap`: wrapper for mmap + mbind + madvise + prefault
* `NumaArena`: per-node bump/slab allocator for fixed-size structs
* `PageFramePool`: buffer frames allocated per node

Key API sketches:

* `NumaRegion::anon(size, policy, HugePageMode, PrefaultMode)`
* `NumaRegion::file(fd, offset, size, policy, ...)`
* `region.prefault(strategy)` (sequential touch or parallel touch)
* `region.migrate(to_policy)` (best-effort)

#### Huge pages

Provide explicit modes:

* `HugePageMode::TransparentOn | TransparentOff`
* `HugePageMode::Explicit2MB | Explicit1GB` (if system configured)

Also expose a “safe default”: THP on for large anonymous regions, off for tiny.

#### Prefault / first-touch strategies

This is big:

* `Prefault::None`
* `Prefault::SerialTouch`
* `Prefault::ParallelTouch { node_workers: usize }`

Parallel prefault means: pin a worker per node, stripe the region, touch pages locally.

This is how you **force** locality for mmaps and large anon regions.

---

### 4) `neul_sched` — NUMA executor + work stealing topology

**Goal:** enforce locality in scheduling.

Design:

* one worker pool per NUMA node
* each node has a local deque
* steal order: same LLC → same socket → other socket last
* optional “home node” tagging for jobs/morsels

Primitives:

* `NumaExecutor::new(topo, policy)`
* `exec.submit(node_id, job)`
* `exec.submit_local(job)`
* `exec.submit_home(HomeKey, job)` where `HomeKey` maps to a node (e.g., table partition id)
* `MorselPlanner` that produces morsels tagged with node affinity

Hard-mode feature:

* `no_global_queue` (compile-time / runtime assertion)
* `cross_node_steal_budget` (to prevent accidental remote storms)

---

### 5) `neul_sharded` — NUMA-local shared structures

**Goal:** delete global contention by construction.

Offer:

* `NumaSharded<T>`: one instance per node
* `CachePadded` wrappers
* per-node counters/histograms with periodic reduction
* `NumaHashMap` (sharded hashmap for hot registries: buffer directory, plan cache, etc.)

Important: expose “read mostly” patterns:

* RCU-style pointer swaps for catalogs
* epoch-based reclamation (crossbeam epoch)

---

### 6) `neul_io_locality` — NVMe/NIC locality helpers

For big DB boxes with multiple NVMe drives:

* choose IO worker threads pinned to the NUMA node closest to that device
* map file shards / WAL segments to drives and keep the hot path local

APIs:

* `closest_node_for_block_device("/dev/nvme0n1") -> NodeId`
* `IoPool::for_device(dev).submit(read/write)`

This is optional but extremely powerful.

---

### 7) `neul_perf` — locality observability (make NUMA measurable)

Hard mode needs feedback.

Expose:

* per-node allocation bytes, faults, prefault time
* per-node queue depths
* cross-node steals
* remote-page estimate (best-effort)
* bandwidth-ish counters (approx)

Implementation options:

* **cheap**: internal counters + `get_mempolicy` sampling + queue stats
* **hard**: perf events (uncore IMC counters, NUMA events) — more complex but doable on Linux with privileges

Even the cheap layer gives you “are we going remote?” signals.

---

## How mimalloc fits (yes, use it)

Use **mimalloc as global allocator** for:

* small/medium allocations
* per-query ephemeral objects
* general runtime overhead

But for hard mode you still do:

* buffer pool frames: `NumaRegion` / `PageFramePool`
* join hash tables: `NumaRegion` per node
* sort scratch: `NumaRegion` per node
* column fragments: `Interleave` policy for scan-heavy read-only chunks, or `Bind` if partitioned per node

So: mimalloc everywhere, explicit NUMA memory for the stuff that matters.

---

## Hard-mode “defaults” you ship (opinionated)

* Start DB workers pinned per node.
* Allocate:

  * buffer frames: `Bind(node)` (strict locality)
  * WAL: `Preferred(primary_node)` + IO thread pinned to WAL’s device node
  * scan-heavy read-only columns: `Interleave(all_nodes)`
  * join build tables: `Bind(node)` and hash-partition inputs by node
* Prefault:

  * buffer pool prefaulted in parallel per node at startup (or on first expansion)
* Stealing:

  * allow stealing, but only after local queue empty and under a budget

This makes “NUMA discipline” an invariant, not advice.

---

## What this gives your DB, concretely

With this library underneath, the DB can do:

* “this partition lives on node 1” → schedule morsels on node 1
* “this hash table is node 0” → build/probe locally
* “this mmap region should interleave” → ensure scan bandwidth scales
* “we’re going remote” → detect and adjust

That’s how you get the **128–192 core** scaling story.

---

## Biggest sharp edges (and how we handle them)

1. **Privileges / system config**

   * huge pages, perf counters, mlock can need sysadmin knobs
   * library should degrade gracefully: “hard mode requested but unavailable; running in soft-hard mode”

2. **File-backed mmaps and NUMA**

   * placement is not automatic; you need prefault-touch after mapping
   * you must be explicit about it

3. **Kernel scheduler surprises**

   * pinning helps, but you must avoid spawning random helper threads unpinned
   * library should provide “spawn everything through me” patterns

---

## If you want the next artifact

I can write the *actual* Rust public API skeleton (traits/structs) for `neul_mem` + `neul_sched` (the two that matter most), including:

* `NodeMask` representation
* mmap + mbind wrapper safety model
* parallel prefault implementation
* pinned worker bootstrap

…and it’ll be ready to drop into your DB workspace.

