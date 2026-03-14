# Versioning and Compatibility Policy

This document describes numaperf's versioning policy, minimum supported Rust version (MSRV), and compatibility guarantees.

## Semantic Versioning

numaperf follows [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** version (X.0.0): Breaking API changes
- **MINOR** version (0.X.0): New features, backward compatible
- **PATCH** version (0.0.X): Bug fixes, backward compatible

### Pre-1.0 Versioning

While numaperf is at version 0.x.y:

- Minor version bumps (0.X.0) may contain breaking changes
- Patch version bumps (0.0.X) are always backward compatible
- We will provide migration guides for all breaking changes

## Minimum Supported Rust Version (MSRV)

**Current MSRV: Rust 1.70**

### MSRV Policy

1. The MSRV will only be increased in minor version bumps
2. We aim to support the current stable Rust and at least the previous 4 releases
3. MSRV increases will be documented in release notes
4. We consider MSRV increases to be non-breaking changes

### Checking MSRV

The MSRV is specified in the workspace `Cargo.toml`:

```toml
[workspace.package]
rust-version = "1.70"
```

## API Stability

### Stable APIs

The following are considered stable and follow semver:

- All public types, traits, and functions in the main `numaperf` crate
- Error types and variants in `NumaError`
- Configuration enums (`HardMode`, `MemPolicy`, `StealPolicy`, etc.)
- Core types (`NodeId`, `CpuSet`, `NodeMask`, `Topology`)

### Experimental APIs

APIs marked with the following are considered experimental:

- `#[doc(hidden)]` attributes
- Items behind `unstable` feature flags
- Items in modules named `internal` or `private`

Experimental APIs may change or be removed in any release.

### Platform-Specific APIs

Some APIs are only available on specific platforms:

- Memory binding (`mbind`) requires Linux with libnuma
- CPU affinity requires Linux `sched_setaffinity`
- Device locality requires Linux sysfs

Unavailable APIs on a platform will either:
- Not compile (with clear error messages)
- Return `NumaError::NotSupported`

## Breaking Changes

### What Constitutes a Breaking Change

- Removing public items (types, functions, methods, fields)
- Changing function signatures (parameters, return types)
- Changing the semantics of existing behavior
- Adding required trait bounds
- Changing error variants that users might match on

### What Is NOT a Breaking Change

- Adding new public items
- Adding new methods to existing types
- Adding new error variants
- Performance improvements
- Bug fixes that correct incorrect behavior
- MSRV increases
- Dependency version updates

## Deprecation Policy

1. Deprecated items will be marked with `#[deprecated]`
2. Deprecation warnings will include migration guidance
3. Deprecated items will remain for at least 2 minor releases
4. Deprecated items will be removed in the next major release

Example deprecation:

```rust
#[deprecated(since = "0.2.0", note = "Use `new_method()` instead")]
pub fn old_method(&self) { ... }
```

## Crate Organization

The numaperf workspace consists of multiple crates:

| Crate | Stability | Description |
|-------|-----------|-------------|
| `numaperf` | Stable | Facade crate, recommended entry point |
| `numaperf-core` | Stable | Core types and errors |
| `numaperf-topo` | Stable | Topology discovery |
| `numaperf-affinity` | Stable | Thread pinning |
| `numaperf-mem` | Stable | Memory placement |
| `numaperf-sched` | Stable | Work scheduling |
| `numaperf-sharded` | Stable | Sharded data structures |
| `numaperf-io` | Stable | Device locality |
| `numaperf-perf` | Stable | Observability |

### Using Individual Crates

You may depend on individual crates, but we recommend using the main `numaperf` crate:

```toml
# Recommended
[dependencies]
numaperf = "0.1"

# Also valid, but less convenient
[dependencies]
numaperf-core = "0.1"
numaperf-topo = "0.1"
```

All workspace crates share the same version number and are released together.

## Release Process

1. **Development**: Features and fixes are developed on feature branches
2. **Review**: Changes are reviewed and merged to main
3. **Changelog**: CHANGELOG.md is updated with all changes
4. **Version Bump**: Version is bumped in workspace Cargo.toml
5. **Release**: All crates are published together

## Reporting Issues

If you encounter:

- **API breaking changes** without a major version bump
- **Undocumented breaking changes**
- **Missing deprecation warnings**

Please report them at: https://github.com/numaperf/numaperf/issues

## Version History

### 0.1.0 (Initial Release)

- Core types: `NodeId`, `CpuSet`, `NodeMask`
- Error handling: `NumaError`
- Topology discovery
- Thread pinning with `ScopedPin`
- Memory placement with policies
- NUMA-aware executor
- Sharded data structures
- Device locality
- Observability and diagnostics
- Hard mode enforcement
- Capability detection

## Future Compatibility

We are committed to:

1. Providing stable, well-documented APIs
2. Minimizing breaking changes
3. Providing clear migration paths
4. Supporting a reasonable range of Rust versions
5. Maintaining Linux as the primary supported platform

For questions about compatibility, please open a discussion on GitHub.
