# AGENTS.md — Guidelines for AI Agents

This repository is a **Rust workspace** with async code (Tokio-based). Use these rules when making changes.

## Workspace & dependencies

- Prefer adding dependencies at the **workspace root** `Cargo.toml` under `[workspace.dependencies]`.
- In crate `Cargo.toml` files, reference them via **workspace dependency resolution**:
  - `dep = { workspace = true }`
  - `dep = { workspace = true, features = ["..."] }`
- Keep versions **centralized** in the workspace; avoid per-crate version drift.
- Only enable features you need; document why new deps/features are required.

## Async Rust best practices (Tokio)

- Never block the runtime: avoid `std::thread::sleep`, blocking IO, heavy CPU work on async tasks.
  - If unavoidable, use `tokio::task::spawn_blocking` (and keep the critical section small).
- Propagate cancellation: respect `.await` cancellation and avoid “fire-and-forget” unless justified.
  - Prefer structured concurrency (`JoinSet`, `try_join!`) and ensure tasks are joined/dropped cleanly.
- Add timeouts to external IO (`tokio::time::timeout`) where hangs are possible.
- Limit concurrency and backpressure: prefer bounded channels/streams; avoid unbounded buffering.
- Prefer explicit lifetimes/ownership and minimal cloning; use `Arc` intentionally.
- Avoid `unwrap()`/`expect()` in library/runtime paths; return typed errors instead.
- Keep public APIs small and well-documented; maintain backwards-compatible behavior unless required.

## Error handling & logging

- Use structured errors (`thiserror`/custom enums) and include actionable context.
- Preserve sources (`#[source]`, `std::error::Error::source`) for troubleshooting.
- Use structured logging (`tracing`) and avoid leaking secrets (API keys, cookies, tokens).

## Code style & project hygiene

- Match existing patterns in each crate (`client`, `parse`, `sse`, `upload`, etc.).
- Prefer small, testable functions; add unit/integration tests for non-trivial behavior changes.
- Update docs/examples when API behavior changes.

## Required after-change checks

After writing or modifying code, **always** run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets
```

