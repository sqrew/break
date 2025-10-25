# Contributing to breakrs

Thanks for your interest in contributing! This project follows the Unix philosophy: do one thing and do it well.

## Getting Started

1. Fork and clone the repository
2. Make sure you have Rust 1.70+ installed
3. Run tests: `cargo test`
4. Run linter: `cargo clippy -- -D warnings`
5. Format code: `cargo fmt`

## Development Workflow

```bash
# Make your changes
git checkout -b my-feature

# Test everything
cargo test
cargo clippy -- -D warnings
cargo fmt

# Build release to verify
cargo build --release

# Commit and push
git add .
git commit -m "Brief description of changes"
git push origin my-feature
```

## Pull Request Guidelines

- **Keep it simple** - This project values concision over features
- **No new dependencies** unless absolutely necessary
- **All tests must pass** - CI runs tests on Linux, macOS, and Windows
- **Code must be formatted** - Run `cargo fmt` before committing
- **No Clippy warnings** - Run `cargo clippy -- -D warnings`
- **Update CHANGELOG.md** if your change is user-facing

## What to Contribute

**Good contributions:**
- Bug fixes
- Performance improvements
- Cross-platform compatibility fixes
- Better error messages
- Documentation improvements
- Test coverage

**Think twice before contributing:**
- New features (open an issue first to discuss)
- Configuration files (we intentionally avoid configs)
- Breaking changes to the CLI interface
- Dependencies that bloat the binary size

## Code Style

- Follow Rust conventions
- Use `cargo fmt` (enforced by CI)
- Keep functions focused and reasonably sized
- Add doc comments for public APIs
- Use named constants instead of magic numbers

## Testing

- Add tests for new functionality
- Ensure all 49+ existing tests pass
- Test on your platform (CI will test others)
- Consider edge cases

## Architecture

```
src/
├── main.rs      # CLI interface and command handlers
├── parser.rs    # Natural language duration parsing
├── database.rs  # JSON storage with file locking
└── daemon.rs    # Background process for notifications
```

The project intentionally keeps things simple:
- No async runtime
- No database engine
- No config files
- No logging framework
- Simple JSON file storage

## Questions?

Open an issue! We're happy to help.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
