# Contributing

## Local checks

Run the same Rust checks used by CI:

```bash
cargo fmt --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
```

The local release binary is accepted by the launcher, so hooks can be tested directly from a source checkout.

## Releasing

1. Update the version in `Cargo.toml` and `.codex-plugin/plugin.json`.
2. Run the local checks.
3. Create and push a matching tag such as `v0.1.0`.

The release workflow builds all supported targets, publishes the complete plugin marketplace to the generated `marketplace` branch, and creates a GitHub release with release notes. Do not edit the `marketplace` branch manually.
