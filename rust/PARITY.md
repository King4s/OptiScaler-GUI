# Parity checklist — Rust rewrite vs Python v0.5.2

Status per functional area, with the evidence behind each claim.
Verified = automated test or measured comparison; Manual = needs a human pass.

| Area | Status | Evidence |
|---|---|---|
| Steam scanning (ACF/VDF, multi-library, registry roots) | ✅ Verified | 28/28 games identical to Python on dev machine (`examples/scan_cli.rs` diff); unit tests both VDF formats |
| Epic / GOG / Xbox / Heroic / manual-folder scanning | ✅ Verified | Same 28/28 diff (Epic 1, Xbox 7); Heroic all-4-backends unit tests ported from Python suite |
| Engine / anti-cheat / OptiScaler-installed detection | ✅ Verified | Single-walk FolderFacts parity tests; badge spot-check via screenshots |
| Install (download, SHA256 digest, extract, payload, manifest) | ✅ Verified | Real v0.9.3 install round-trip; digest tests incl. mismatch |
| **Cross-app manifest contract (schema v1)** | ✅ Verified | Rust install → real Python app uninstalled cleanly (0 leftovers); Python-written fixture parsed in tests |
| Update flow (target reuse, ini backup, stale cleanup) | ✅ Verified | winmm.dll reuse test; user-modified ini preserved in timestamped backup; v0.9.2→v0.9.3 |
| Uninstall (manifest-based + legacy fallback) | ✅ Verified | Unit tests incl. Python-manifest fixture; real uninstall left game files intact |
| Artwork pipeline (cache layout, CDN, Store API, SteamSpy) | ✅ Verified | Python-compatible `cache/game_images` + `steam_app_list.json`; screenshots show real art |
| AppID matching ladder | ⚠️ Partial | Exact/normalized/suffix-strip/token-subset ported + tested. **Deviation: difflib fuzzy last-resort not ported** |
| OptiScaler.ini editor (parse/types/write/backup) | ✅ Verified | 287/287 section\|key\|type\|value identical to Python reader on the real 0.9.3 ini |
| Editor UI (widgets, search, dirty guard, Auto Settings) | 🖐 Manual | Compiles + logic tested; needs a human click-through on a real install |
| GPU detection for Auto Settings | ⚠️ Simplified | wgpu adapter vendor id (no PowerShell/wmic). **Deviation: no GPU name/VRAM heuristics — FP16 recommendation always on** |
| Global settings (language/theme/cache/excluded drives) | ✅ Verified | Config roundtrip test proves Python fields survive; live language switch |
| i18n en/da/pl | ✅ Verified | Canonical JSON files embedded; no-extra-keys test; new keys added to all three |
| OptiScaler update badge + GUI self-update check | ✅ Verified | Version-comparison tests; badge logic in detail panel/sidebar |
| Progress reporting | ✅ Verified | Per-game stage progress (download %, payload count) via event channel |
| Portable single exe | ✅ Verified | 6.9 MB release build, launch-verified; no bundled 7z.exe needed (BCJ2 in pure Rust, hash-identical to 7z.exe) |
| Shader effects + toggle + reduced motion | ✅ Verified | Launch with pass active renders correctly; zero-idle repaint policy; SPI honored |

## Known gaps (tracked for beta feedback)
- No on-disk debug log file yet (in-app log ring only; Python writes `optiscaler_gui_debug.log`)
- Heroic scanning verified against fixtures + store formats, not against a live Heroic install on the dev machine (none present)
- Uninstaller `Remove OptiScaler.bat` content matches Python's shape but hasn't been executed as a standalone script in verification
- GPU auto-recommendation simplifications listed above
