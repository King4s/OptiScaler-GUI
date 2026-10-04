# OptiScaler-GUI contributor instructions

This repository is Rust-only. Read [CLAUDE.md](../CLAUDE.md) for architecture, safety rules and commands.
Run fmt, strict all-target Clippy and locked workspace tests from `rust/`.
Embedded translations and community game data live in `rust/crates/opticore/data/`.
All blocking work belongs on workers; hardware recommendations never automatically alter a game or INI.
Preserve explicit rendering-GPU choice, community consent, resolver/manifest/rollback safety and privacy.
Historical Python release evidence is not current build guidance.
