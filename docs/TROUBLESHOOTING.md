# Troubleshooting

## 2026.10.0 hardware/runtime changes

Hardware collection runs on a worker at every startup. Use the always-visible Refresh button if facts change;
then choose the rendering GPU for each game again. Unknown facts and recommendations are not compatibility proof.
Delete hardware in Settings to cancel a pending collection and clear its local profile; next startup collects again.
Community RDNA2 installs require fresh explicit RX 6000 choice and consent for manual updates. Automatic updates
refuse them to prevent an unapproved runtime swap. Verification failures leave game files unchanged.
See the [feature guide](hardware-runtime.md) for pins, privacy and warnings.


This guide describes the Rust `2026.9.0` discovery/artwork features. See the [verification record](releases/2026.9.0-verification.md) for test evidence and limits. Neither a detected install nor a log message guarantees that a particular game, GPU, driver, or configuration works.

## Overlay does not open

- Start the game with OptiScaler enabled, then press **Insert**. On non-US keyboard layouts, try **Alt+Insert**; the GUI displays this hint when it detects a non-US layout.
- Check that the game was launched with the OptiScaler proxy enabled, not with the **Play without OptiScaler** action. The game card can indicate when OptiScaler is bypassed.
- Verify the selected EXE and install folder shown in the game details. An incorrect proxy target can prevent the game from loading OptiScaler at all.
- RTSS (RivaTuner Statistics Server) or another overlay/hook may conflict with the OptiScaler overlay. For diagnosis, close RTSS and other overlay/injection tools, restart the game, and test again. This is a troubleshooting isolation step, not a confirmed universal incompatibility or a claim that RTSS must always be disabled.
- Runtime options are configured in OptiScaler's own overlay. The GUI INI editor only previews and saves file changes; restart the game after saving. It does not guarantee that a setting is valid or supported by the game.

## Installation target is missing or looks wrong

- The resolver requires a usable game executable inside the selected game root. Launcher, setup, helper, anti-cheat, and other known non-game executables are filtered out.
- If multiple candidates remain, use **Select actual game EXE...** and choose the game's executable, not a launcher or utility. Review both the displayed EXE and target folder before installing.
- Satisfactory is a special documented layout: the game executable is a `FactoryGame*Win64-Shipping.exe` under `FactoryGame/Binaries/Win64`, but the OptiScaler proxy target is `Engine/Binaries/Win64`. These paths are intentionally different. Check the installed game version and upstream OptiScaler compatibility guidance; if the expected Engine folder is absent, do not force an install elsewhere.
- A persisted executable choice is stored relative to the game root. If it no longer exists or the layout changed after a game update, select the current game EXE again.

## Update or uninstall is refused

- Manifest v2 records the hashes of files managed by this app and any originals it backed up. If an owned file was changed or removed, update/uninstall stops to avoid overwriting or deleting unexpected data. Restore the file from a trusted backup or inspect it manually before proceeding.
- A legacy v1 manifest (including installs made by the older Python app) does not prove per-file ownership. A missing manifest proves nothing either. The app deliberately refuses guessed cleanup; do not delete proxy DLLs or payload folders based only on their names.
- If the UI reports an installation lock, ensure no other installer operation is running. Do not remove `.optiscaler-gui.lock` while an operation may still be active.
- A rollback-incomplete error identifies a recovery snapshot location. Preserve that directory and inspect the app log before attempting another install; do not clean up the snapshot as ordinary temporary data.

## Loaded status or log evidence looks inconsistent

- DLL names in the observation panel are hints, not ownership proof and not proof that Windows loaded the DLL into the game.
- The `Init done` loaded indicator is parsed only for recorded OptiScaler v0.9.4 installs. It is derived from `OptiScaler.log` beside the selected game executable, not the proxy target folder. A matching line can be left over from an earlier launch; compare the displayed log modification time with the latest test and reproduce after starting the game.
- If there is no matching line, the result is unknown rather than proof that OptiScaler failed to load. Other versions, malformed/oversized logs, or inaccessible files do not produce a positive result.
- The GUI application log is under `logs/optiscaler-gui.log` beside the app executable; rotated output is `logs/optiscaler-gui.old.log`, and panic details are written to `logs/crash.log`. Use the Log screen or its folder button to locate logs. Review them before sharing because paths and diagnostic context may be sensitive.

## Report a reproducible problem

Use the game's **Compatibility report** section to select your test result, preview the complete JSON and choose **Save report**. The report includes limited hardware, store/build and OptiScaler version fields, plus separate installed, loaded-log and user-test status. It omits paths, filesystem-derived game names and raw logs; unknown values remain unknown. Review the preview before sharing because a custom GPU name might still contain personal text. Include reproduction steps separately. Attach logs or the selected EXE/target only after checking them for private information. Compatibility is not established for all games. Fatekeeper was reported working by the user on 2026-09-25. That remains valid feedback; no fresh-candidate manual test is claimed, and the absence of that retest does not establish a failure.

## A game is missing from the library

- Rescan after launcher installation or metadata changes. Discovery depends on local installed-game records and existing directories; it cannot guarantee every game will appear.
- Amazon uses its local installed-game SQLite database; Ubisoft uses launcher install registry keys; EA uses registry product IDs and installerdata XML in known or registry-linked locations; Battle.net uses Agent `product.db`. Incomplete, stale or inaccessible metadata can be skipped with warnings. See [store format limits](store-formats.md).
- Add a specific local game-library folder, enable it and rescan. Removing a configured root only removes that scan setting; it does not delete games. A game may still appear through another enabled source.
- Custom scanning is bounded. Drive, profile and system roots, network roots and link/reparse-point traversal are rejected. Choose a narrower real folder when a scan reaches its work limit.
- Manual game selection remains available. A discovered store ID or folder does not bypass the normal executable and install-target review.

## Artwork is missing, wrong or remains landscape

- Cards preserve image aspect ratio and fit the whole cover. Empty space around a landscape image in a portrait area is expected.
- The new cache separates installs by platform, observed store identity and normalized path. Existing legacy files remain readable and untouched, but old title-only art can still show an ambiguous match.
- An identified Steam game can upgrade legacy landscape artwork using local or hosted portraits. Missing, invalid or wrong-ID images fall through to other sources, cached art or icons. Artwork does not affect game identity, compatibility or install targets.
- Use the artwork controls to choose a local image, inspect its source, reset the current selection or refresh automatic sources. Refresh preserves a selected local or SteamGridDB image; reset the selection first if you want automatic artwork again. Reset does not delete the original image.
- A saved miss avoids retrying downloads on every render. Refresh explicitly retries automatic sources when connectivity or metadata changes and preserves the last good artwork if replacement fails.
- Clear cache only clears recognized automatic cache entries. Legacy files and entries with selected-art history are preserved; do not delete the entire image-cache folder to force a portrait upgrade.

## SteamGridDB artwork

- Give explicit consent and enter your own API key for the session, then search. Choose the correct game from the results and then the image; no first result is selected automatically.
- Lookup sends an explicit Steam app ID when available, or the game title for search, to SteamGridDB. The key is held in memory for API requests, not saved as a setting or sent with image downloads.
- Automatic cover fetching and refresh never invoke SteamGridDB. Automatic store/Steam artwork downloads can still occur; choosing not to use SteamGridDB does not disable those sources.
- If the service is unavailable, the key is rejected or no suitable match appears, keep the existing art or choose a local image. Automatic SteamSpy title matching is not used.
- See [cover artwork details](cover-art.md). Authenticated SteamGridDB testing was not performed because no API key was available; startup smoke does not establish manual visual or gameplay verification.
