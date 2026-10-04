# Hardware recommendations and community RDNA2 runtime

**Implemented in current Rust source, unreleased.** No new published binary or gameplay guarantee is claimed.

## Startup, refresh and privacy

Every app startup collects a fresh local DXGI hardware snapshot on a worker regardless of update-check settings.
Recommendations and manual refresh are visible at the bottom on every screen. Settings shows GPU names/vendor,
dedicated/shared memory, driver, Windows build, RAM, collection time and collection warnings when available.
Partial/unknown facts stay unknown. Recommendations are advisory and never change INI settings or install files.

The local profile is stored beside the executable in `cache/local-profiles.json`. Settings → Delete hardware profile
cancels a pending result and clears hardware/per-game GPU selections; it does not erase independent test results.
A manual refresh or next app startup collects again. An unreadable profile is not overwritten without explicit deletion.
No hardware upload is added; compatibility reports still require preview and explicit save before sharing.

## Family guidance

| Family | Guidance, conditional on game support |
|---|---|
| Radeon RX 6000 (RDNA2) | Optional pinned community FSR 4.1.1b INT8 RDNA2 fix |
| RX 7000 / RX 9000 (RDNA3/4) | Keep official FSR 4 |
| NVIDIA RTX | Prefer native DLSS |
| GTX / older Radeon RX | Consider XeSS or FSR 3.1 |
| Intel Arc | Prefer XeSS |
| Unknown/APU/ambiguous name | Verify actual rendering GPU; no automatic patch |

Name-based guidance is not capability measurement. Multiple adapters get separate guidance; no first/default/GUI
adapter is assumed to render a game. Explicitly choose the game's rendering GPU in game details after startup/refresh.
DXGI snapshot IDs are enumeration-local; refresh clears saved choices and pending/stale snapshots cannot opt in.

## Installing the optional RX 6000 runtime

1. Choose the RX 6000 rendering GPU explicitly; a single detected RX 6000 is still only a suggestion.
2. Review the executable/target and normal anti-cheat warning. Install/update asks separately whether to use the
community runtime; No keeps the official payload. Existing current installs also expose a community runtime button.
3. Yes opts in for that operation only. This is not an official AMD or OptiScaler runtime; compatibility/stability are
not guaranteed. Do not use mods in anti-cheat/online games. The app provides no bypass.
4. The worker downloads only the pinned community asset in a separate cache, verifies archive SHA256/size, extracts
freshly, verifies the exact DLL size/hash and privately stages/revalidates before touching the game.
5. Only `amd_fidelityfx_upscaler_dx12.dll` in the game payload is substituted; official extraction stays unchanged.
The normal transaction records ownership/backups and atomically replaces files. Manifest `fsr_runtime` records
source, asset, archive/DLL hashes, DLL size, community consent and selected GPU name (not loaded-GPU proof).

Automatic updates refuse existing community installs: manual selection and consent are required again.
Use Restore official runtime to replace the community runtime even when no newer release exists. Manual official updates remove community provenance while using the official runtime; restoration refuses a payload without an official replacement DLL. Owned update/uninstall rules
remain in force; modified/foreign files are never guessed away, and original replaced files are restored on uninstall.
Download/extraction/verification errors occur before game mutations and preserve the existing install.

## Pinned source

- Release: https://github.com/the3rdparty1917/fsr4xyz/releases/tag/4.1.1b
- Asset: `FSR_4.1.1b_INT8_with_RDNA2_fix.7z` (3,456,158 bytes)
- Archive SHA256: `66e9a818e0c914def7712c8dac06b08e64a64dbcfe77f3162d43ea6de93869ff`
- Member: `4.1.1b/amd_fidelityfx_upscaler_dx12.dll`
- DLL SHA256: `0dd77d9c78d1ef9bc330cf4697ab3ffe24bc1aa7850e4130263dc922107fbd75`
- DLL size: 34,013,696 bytes

Pins are code-reviewed constants, not a mutable latest-community-release lookup. No other patch is inferred for
RDNA3/4, NVIDIA, Arc, unknown/APU or mixed hardware without an explicit eligible per-game choice.
