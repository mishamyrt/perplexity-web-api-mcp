# Contributing Guide

Thank you for your interest in contributing to Perplexity Web API MCP Server! This document provides guidelines and instructions for contributing.

## Requirements

- **Rust 1.91+** (edition 2024)
- **Cargo** (comes with Rust)
- A Perplexity AI account (for testing)

## Getting Started

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/perplexity-web-api-mcp.git
cd perplexity-web-api-mcp
```

### 2. Build the Project

#### macOS / Linux

```bash
cargo build --workspace --all-targets
cargo build -p perplexity-web-api-mcp --all-targets --features streamable-http
```

To build the optimized MCP binary directly:

- `cargo build --profile dist --bin perplexity-web-api-mcp` builds the smaller `stdio`-only binary.
- `cargo build --profile dist --bin perplexity-web-api-mcp --features streamable-http` enables the optional Streamable HTTP transport.

#### Windows

Building on Windows requires additional tools because this project depends on BoringSSL (via the `rquest` crate).

**Prerequisites:**

- [Rust](https://rustup.rs)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **C++ desktop development** workload
- CMake, NASM, Ninja, and LLVM (libclang)

**Using the build script:**

The included `build.ps1` script automatically downloads and sets up all required build tools into a local `.build-tools` directory:

```powershell
# First build (downloads dependencies automatically)
powershell -ExecutionPolicy Bypass -File build.ps1

# Skip dependency download (if tools are already installed system-wide)
powershell -ExecutionPolicy Bypass -File build.ps1 -SkipDeps

# Clean build (removes .build-tools and target directories)
powershell -ExecutionPolicy Bypass -File build.ps1 -Clean
```

> **Note:** The build must use `--release` mode. Debug builds cause a CRT mismatch between BoringSSL (`/MDd`) and Rust (`/MD`), resulting in linker errors.

### 3. Run Tests

```bash
cargo test --workspace --lib
```

## Project Structure

```
perplexity-web-api-mcp/
├── crates/
│   ├── perplexity-web-api/       # Core API client library
│   │   ├── src/
│   │   │   ├── auth.rs           # Auth cookie types, cookie-name constants, REDACTED_SECRET
│   │   │   ├── client.rs         # HTTP client and request handling
│   │   │   ├── config.rs         # API configuration constants (base URL, timeouts)
│   │   │   ├── error.rs          # Error types
│   │   │   ├── http.rs           # Shared HTTP response helpers (ensure_success_response)
│   │   │   ├── models.rs         # Model preference types and name constants
│   │   │   ├── parse.rs          # Response parsing
│   │   │   ├── sse.rs            # Server-Sent Events stream handling
│   │   │   ├── types.rs          # Request/response types and request_requires_authentication
│   │   │   └── upload.rs         # File upload functionality
│   │   └── examples/             # Usage examples
│   └── perplexity-web-api-mcp/   # MCP server binary
│       └── src/
│           ├── auth.rs           # Auth resolution (env → saved config → interactive → tokenless)
│           ├── config.rs         # Saved auth config persistence (load/save with file permissions)
│           ├── main.rs           # Entry point and model env var resolution
│           ├── server.rs         # MCP tool implementations
│           ├── setup.rs          # Interactive first-run setup wizard
│           ├── test_utils.rs     # Shared test helpers (TempDir)
│           └── tty.rs            # TTY detection for interactive-mode guard
├── Cargo.toml                    # Workspace configuration
└── AGENTS.md                     # AI agent guidelines
```

## Development Guidelines

### Workspace Dependencies

- Add new dependencies to the **workspace root** `Cargo.toml` under `[workspace.dependencies]`
- Reference them in crate `Cargo.toml` files via workspace resolution:

```toml
[dependencies]
new-dep = { workspace = true }
# or with features:
new-dep = { workspace = true, features = ["feature-name"] }
```

### Async Rust Best Practices

This project uses **Tokio** for async runtime. Follow these guidelines:

- **Never block the runtime**: avoid `std::thread::sleep`, blocking IO, or heavy CPU work on async tasks
  - Use `tokio::task::spawn_blocking` if blocking is unavoidable
- **Propagate cancellation**: respect `.await` cancellation points
- **Add timeouts**: use `tokio::time::timeout` for external IO operations
- **Limit concurrency**: prefer bounded channels/streams; avoid unbounded buffering
- **Minimize cloning**: prefer explicit lifetimes/ownership; use `Arc` intentionally
- **No panics in library code**: avoid `unwrap()`/`expect()` in runtime paths; return typed errors

### Error Handling

- Use structured errors with `thiserror`
- Include actionable context in error messages
- Preserve error sources with `#[source]` for debugging
- Never leak secrets (API keys, cookies, tokens) in error messages or logs

### Code Style

- Match existing patterns in each module
- Prefer small, testable functions
- Document public APIs with doc comments
- Update examples when API behavior changes

## Before Submitting

### Required Checks

Always run these commands before committing:

```bash
# Format code
cargo fmt --all

# Lint with clippy (must pass with no warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build all targets
cargo build --workspace --all-targets

# Verify the optional Streamable HTTP transport build
cargo build -p perplexity-web-api-mcp --all-targets --features streamable-http

# Run deterministic unit coverage
cargo test --workspace --lib
```

Authenticated e2e coverage is opt-in because it depends on live Perplexity session cookies:

```bash
PERPLEXITY_SESSION_TOKEN="..." \
PERPLEXITY_CSRF_TOKEN="..." \
cargo test -p perplexity-web-api --test integration -- --ignored --test-threads=1
```

### Commit Messages

- Use clear, descriptive commit messages
- Start with a verb in imperative mood: "Add", "Fix", "Update", "Remove"
- Keep the first line under 72 characters
- Reference issues when applicable: "Fix #123"

Examples:
```
Add timeout configuration to ClientBuilder
Fix SSE stream parsing for malformed events
Update error types for better diagnostics
```

## Pull Request Process

1. **Create a feature branch** from `master`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the guidelines above

3. **Run all checks** to ensure code quality

4. **Push and create a PR** with a clear description:
   - What changes were made
   - Why the changes are needed
   - How to test the changes

5. **Address review feedback** promptly

## Testing

### Running Examples

Examples require authentication tokens. Set environment variables:

```bash
export PERPLEXITY_SESSION_TOKEN="your-session-token"
export PERPLEXITY_CSRF_TOKEN="your-csrf-token"
```

`PERPLEXITY_SESSION_TOKEN` should be copied from the browser cookie named `__Secure-next-auth.session-token`.

Then run:

```bash
cargo run --example basic
cargo run --example streaming
```

### Adding Tests

- Add unit tests in the same file as the code being tested
- Add integration tests in `tests/` directory if needed
- Ensure tests are deterministic and don't require external services

## Questions?

If you have questions or need help, feel free to open an issue for discussion.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
