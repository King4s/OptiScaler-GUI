# OptiScaler-GUI Rust beta: completed and remaining

Status snapshot for `2026.9.0-beta.1 Standard`. This prerelease is not a stable-release checklist approval; implementation progress is not equivalent to validated behavior.

## Implemented in the current tree

- [x] Shared target resolver is used across install, update, launch, and observations. Ambiguous candidates stop for explicit selection; selected executable and target directory are shown before install.
- [x] Satisfactory exception keeps the actual `FactoryGame-*-Win64-Shipping.exe` under `FactoryGame/Binaries/Win64` distinct from the proxy target under `Engine/Binaries/Win64`, even if the installation root is renamed.
- [x] Manifest v2 records owned-file hashes and original backups. Mutations use guarded paths and rollback snapshots, refuse changed/missing owned binaries and new foreign collisions on update, recheck immediately before each write, and do not write through hard-link destinations. Conflicting files are preserved during rollback; edited INI files are preserved.
- [x] Legacy v1 and manifest-less installs do not grant deletion ownership; automatic destructive migration is intentionally withheld.
- [x] Hardware facts, deterministic source-backed advice, local observations, application/crash logs, overlay guidance, and INI change preview exist.
- [x] Observation wording/data treats DLL presence as hints and log `Init done` as limited evidence, not runtime proof.
- [x] A typed report builder and non-overwriting JSON writer pass synthetic privacy tests. The GUI shows the full report before the user chooses to save it; test results stay local per game.
- [x] Local release-binary startup smoke from an isolated test directory remained alive after eight seconds, wrote its startup log and produced no crash log. This does not test gameplay or the overlay.
- [x] Windows Rust CI is configured to run locked tests, Clippy, formatting, release build and a fresh startup smoke. A remote CI run is still pending.
- [x] The `2026.9.0-beta.1` release binary started from a clean local directory, logged its beta version and remained alive without a crash log.
- [x] User reported Fatekeeper working on 2026-09-25. The exact GUI, OptiScaler and game versions, GPU, overlay result and uninstall result were not provided; this is user confirmation, not a documented compatibility matrix.

## Still required

- [ ] Run and record GitHub CI validation for the beta branch; local equivalent Rust checks passed.
- [ ] Perform hands-on packaged-app install, update, rollback, uninstall and legacy-install safety checks on supported Windows systems; the startup smoke alone does not cover these flows.
- [ ] Record a reproducible Fatekeeper test with exact hardware and versions, correct proxy placement, OptiScaler load, overlay behavior and safe uninstall.
- [ ] Validate additional representative game layouts and capture explicit exceptions before making broader compatibility claims.
- [ ] Review beta feedback and known limitations before deciding whether to prepare any public release.

## Release guardrails

- `2026.9.0-beta.1 Standard` is a prerelease for feedback, not a stable 2026.9.0 release or a general compatibility declaration.
- Do not claim all games are compatible. A unique executable or a rule in the compatibility wiki does not establish a working runtime on a user's machine.
- A saved target selection must remain inside the game root. For Satisfactory, verify the displayed EXE and Engine target against the installed game version and the current upstream compatibility guidance.
- Never infer ownership from proxy/DLL presence. For v1 or absent manifests, preserve files and ask for a deliberate manual recovery path rather than deleting guessed files.
- Treat log-based loaded evidence as potentially stale; the JSON report records only the limited status, never raw log content or filesystem-derived game names.
