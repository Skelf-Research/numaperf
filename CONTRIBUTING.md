# Contributing to numaperf

Thank you for your interest in contributing to numaperf!

## How to Contribute

### Reporting Issues

- Use the [GitHub issue tracker](https://github.com/Skelf-Research/numaperf/issues)
- Search existing issues before creating a new one
- Include your OS, kernel version, and Rust version
- Provide a minimal reproduction case if possible

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests (`cargo test --workspace`)
5. Run clippy (`cargo clippy --workspace`)
6. Format code (`cargo fmt`)
7. Commit with a clear message
8. Push and open a pull request

### Code Style

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Add tests for new functionality
- Update documentation as needed

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p numaperf-topo

# Run with verbose output
cargo test --workspace -- --nocapture
```

### Documentation

- Update doc comments for public APIs
- Add examples where helpful
- Keep the MkDocs documentation in sync

## Development Setup

```bash
# Clone the repository
git clone https://github.com/Skelf-Research/numaperf.git
cd numaperf

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run benchmarks
cargo run -p numaperf-bench -- bench
```

## Questions?

- Open a [GitHub discussion](https://github.com/Skelf-Research/numaperf/discussions)
- Email [support@skelfresearch.com](mailto:support@skelfresearch.com)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
