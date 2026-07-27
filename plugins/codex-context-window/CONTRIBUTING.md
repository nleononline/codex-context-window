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
2. Copy `.github/release-notes/TEMPLATE.md` to `.github/release-notes/vX.Y.Z.md`, remove the template marker and guidance, and write the user-facing release notes.
3. Explain the problem, the meaningful changes, and their impact. Pull requests and commits may be linked as supporting detail, but should not be the release summary.
4. Run the local checks and open a pull request.
5. Merge only after the release build matrix and release-notes validation pass.
6. Run the `Release` workflow manually from `master`.

The release workflow builds all supported targets, publishes the complete plugin marketplace to the generated `marketplace` branch, creates the version tag, and uses the matching versioned file as the GitHub release description. Do not create the tag or edit the `marketplace` branch manually.
