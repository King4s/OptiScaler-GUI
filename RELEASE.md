# Rust release procedure

From `rust/`, run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`,
`cargo test --workspace --locked` and `cargo build --release --workspace --locked` on Windows.
The native executable is `target/release/OptiScaler-GUI.exe` (or under CARGO_TARGET_DIR).
There is no Python runtime, PyInstaller build or bundled extractor; runtime 7z decoding is pure Rust.
7-Zip is used only when packaging a release `.7z`.

Version is declared in `rust/Cargo.toml`. Change it only for an authorized release, update CHANGELOG.md,
and write release notes and a verification record under `docs/releases/`. Distinguish source implementation,
startup/file-operation tests and actual gameplay. Verify da/en/pl startup and packaged artifact hashes.

After explicit maintainer approval, create/push the intended CalVer tag (`v20*`).
`.github/workflows/release-rust.yml` tests, builds and publishes EXE/7z/checksums.
`scripts/build-beta-candidate.ps1` and `scripts/verify-beta-package.ps1` support reviewed candidate packaging.
Do not publish/tag as part of ordinary feature implementation. Hardware/RDNA2 features are included in the authorized 2026.10.0 release; release availability is recorded on GitHub.
