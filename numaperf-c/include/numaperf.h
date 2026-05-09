/*
 * numaperf C API
 *
 * Stable C bindings for NUMA topology discovery, thread affinity,
 * and NUMA-aware memory allocation.
 *
 * License: MIT
 * Repository: https://github.com/Skelf-Research/numaperf
 */


#ifndef NPAPERF_H
#define NPAPERF_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Memory placement policy.
 */
typedef enum {
    /**
     * Use the current thread's local NUMA node.
     */
    LOCAL = 0,
    /**
     * Strictly bind to the nodes in the node mask.
     */
    BIND = 1,
    /**
     * Prefer the specified node, with fallback allowed.
     */
    PREFERRED = 2,
    /**
     * Interleave pages across the nodes in the node mask.
     */
    INTERLEAVE = 3,
} NpaMemPolicy;

/**
 * Huge page configuration.
 */
typedef enum {
    /**
     * No huge pages.
     */
    NONE = 0,
    /**
     * Enable transparent huge pages.
     */
    TRANSPARENT_ON = 1,
    /**
     * Disable transparent huge pages.
     */
    TRANSPARENT_OFF = 2,
    /**
     * Explicit 2 MB huge pages.
     */
    EXPLICIT2_MB = 3,
    /**
     * Explicit 1 GB huge pages.
     */
    EXPLICIT1_GB = 4,
} NpaHugePageMode;

/**
 * Memory prefault strategy.
 */
typedef enum {
    /**
     * Do not prefault pages.
     */
    NONE = 0,
    /**
     * Touch pages sequentially from the current thread.
     */
    TOUCH = 1,
    /**
     * Touch pages in parallel using multiple threads.
     */
    PARALLEL_TOUCH = 2,
} NpaPrefault;

/**
 * Opaque handle to a NUMA memory region.
 */
typedef struct NpaRegion NpaRegion;

/**
 * Opaque handle to a discovered NUMA topology.
 */
typedef struct NpaTopology NpaTopology;

/**
 * A set of up to 1024 CPUs, represented as a bitmap.
 *
 * Use `npa_cpuset_add()` and `npa_cpuset_contains()` to manipulate.
 */
typedef struct {
    uint64_t bits[16];
} NpaCpuSet;

/**
 * A set of up to 64 NUMA nodes, represented as a bitmap.
 */
typedef struct {
    uint64_t bits;
} NpaNodeMask;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Get a human-readable description of an error code.
 *
 * Returns a pointer to a static string. The returned pointer is valid for
 * the lifetime of the program and must not be freed.
 *
 * # Arguments
 *
 * * `code` - The error code returned by a numaperf function.
 */
const char *npa_error_string(int32_t code);

/**
 * Get the last error message for the current thread.
 *
 * Returns a pointer to a thread-local C string. The pointer is valid until
 * the next numaperf call on the same thread. It must not be freed.
 */
const char *npa_last_error(void);

/**
 * Discover the system's NUMA topology.
 *
 * Returns an opaque handle on success, or NULL on failure. Check
 * `npa_last_error()` for details on failure.
 *
 * The returned handle must be freed with `npa_topology_free()`.
 */
NpaTopology *npa_topology_discover(void);

/**
 * Free a topology handle obtained from `npa_topology_discover()`.
 *
 * Passing NULL is a no-op.
 */
void npa_topology_free(NpaTopology *topo);

/**
 * Get the number of NUMA nodes.
 */
uint32_t npa_topology_node_count(const NpaTopology *topo);

/**
 * Get the total number of CPUs across all nodes.
 */
uint32_t npa_topology_cpu_count(const NpaTopology *topo);

/**
 * Get the ID of a NUMA node by index.
 *
 * Returns the node ID on success, or `u32::MAX` if `idx` is out of range.
 */
uint32_t npa_topology_node_id(const NpaTopology *topo, uint32_t idx);

/**
 * Get the number of CPUs on a specific node.
 *
 * Returns the CPU count on success, or `0` if the node does not exist.
 */
uint32_t npa_topology_node_cpu_count(const NpaTopology *topo, uint32_t node_id);

/**
 * Get the CPU set for a specific node.
 *
 * Returns `0` on success, or a negative error code. The `cpus` struct
 * must be valid and will be written to.
 */
int32_t npa_topology_node_cpus(const NpaTopology *topo, uint32_t node_id, NpaCpuSet *cpus);

/**
 * Get the NUMA distance between two nodes.
 *
 * Returns the distance value on success, or `u32::MAX` if unavailable.
 * A distance of 10 typically indicates local access.
 */
uint32_t npa_topology_node_distance(const NpaTopology *topo, uint32_t from, uint32_t to);

/**
 * Get the total memory in bytes for a specific node.
 *
 * Returns the memory size on success, or `0` if unavailable.
 */
uint64_t npa_topology_node_memory_bytes(const NpaTopology *topo, uint32_t node_id);

/**
 * Get the NUMA node that contains a specific CPU.
 *
 * Returns the node ID on success, or `u32::MAX` if the CPU is not found.
 */
uint32_t npa_topology_node_for_cpu(const NpaTopology *topo, uint32_t cpu);

/**
 * Create an empty CPU set.
 */
NpaCpuSet npa_cpuset_new(void);

/**
 * Add a CPU to a CPU set.
 */
void npa_cpuset_add(NpaCpuSet *cpus, uint32_t cpu);

/**
 * Remove a CPU from a CPU set.
 */
void npa_cpuset_remove(NpaCpuSet *cpus, uint32_t cpu);

/**
 * Check if a CPU is in a CPU set.
 */
int32_t npa_cpuset_contains(const NpaCpuSet *cpus, uint32_t cpu);

/**
 * Count the number of CPUs in a CPU set.
 */
uint32_t npa_cpuset_count(const NpaCpuSet *cpus);

/**
 * Build a CPU set for a single CPU.
 */
NpaCpuSet npa_cpuset_single(uint32_t cpu);

/**
 * Create an empty node mask.
 */
NpaNodeMask npa_nodemask_new(void);

/**
 * Add a node to a node mask.
 */
void npa_nodemask_add(NpaNodeMask *mask, uint32_t node);

/**
 * Remove a node from a node mask.
 */
void npa_nodemask_remove(NpaNodeMask *mask, uint32_t node);

/**
 * Check if a node is in a node mask.
 */
int32_t npa_nodemask_contains(const NpaNodeMask *mask, uint32_t node);

/**
 * Count the number of nodes in a node mask.
 */
uint32_t npa_nodemask_count(const NpaNodeMask *mask);

/**
 * Build a node mask containing a single node.
 */
NpaNodeMask npa_nodemask_single(uint32_t node);

/**
 * Pin the current thread to a set of CPUs.
 *
 * Returns `0` on success, or a negative error code. The previous affinity
 * is NOT saved. Use `npa_unpin_thread()` with the original CPU set to
 * restore, or call `npa_get_affinity()` before pinning.
 */
int32_t npa_pin_thread(const NpaCpuSet *cpus);

/**
 * Get the current thread's CPU affinity.
 *
 * Returns `0` on success, or a negative error code. The `cpus` struct
 * must be valid and will be written to.
 */
int32_t npa_get_affinity(NpaCpuSet *cpus);

/**
 * Set the current thread's CPU affinity.
 *
 * Alias for `npa_pin_thread()`. Provided for clarity.
 */
int32_t npa_set_affinity(const NpaCpuSet *cpus);

/**
 * Allocate a NUMA-aware memory region.
 *
 * Returns an opaque handle on success, or NULL on failure. Check
 * `npa_last_error()` for details on failure.
 *
 * The returned handle must be freed with `npa_region_free()`.
 *
 * # Arguments
 *
 * * `size` - Size of the region in bytes.
 * * `policy` - Memory placement policy.
 * * `node_mask` - Node mask for `Bind` and `Interleave` policies. Ignored for
 *   `Local` and `Preferred`.
 * * `preferred_node` - Node ID for `Preferred` policy. Ignored for others.
 * * `huge_mode` - Huge page configuration.
 * * `prefault` - Prefault strategy.
 */
NpaRegion *npa_region_alloc(uint64_t size,
                            NpaMemPolicy policy,
                            NpaNodeMask node_mask,
                            uint32_t preferred_node,
                            NpaHugePageMode huge_mode,
                            NpaPrefault prefault);

/**
 * Free a memory region obtained from `npa_region_alloc()`.
 *
 * Passing NULL is a no-op.
 */
void npa_region_free(NpaRegion *region);

/**
 * Get a pointer to the memory region's data.
 *
 * Returns NULL if the region handle is invalid.
 */
void *npa_region_ptr(const NpaRegion *region);

/**
 * Get the size of a memory region in bytes.
 *
 * Returns `0` if the region handle is invalid.
 */
uint64_t npa_region_len(const NpaRegion *region);

/**
 * Get the human-readable name of the memory policy applied to a region.
 *
 * Returns a pointer to a static string, or NULL on error.
 */
const char *npa_region_policy_name(const NpaRegion *region);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* NPAPERF_H */
