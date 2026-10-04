# Technical overview — current Rust application

The Cargo workspace has a GUI crate (egui/wgpu) and `opticore` for testable core logic.
All blocking startup hardware collection, scanning, artwork and installation run on workers.
Resources are embedded from opticore/data (en/da/pl and community game data); no legacy Python source is required.
Archive decoding uses sevenz-rust2, including real BCJ2 payloads; no runtime 7z.exe is needed.
Asset selection currently accepts .7z/.zip names but decoding is 7z, not general ZIP support.

## Installation and ownership

Installer fetches the latest official optiscaler/OptiScaler release and verifies its asset digest when provided.
Resolver confirms executable/target folder, including ambiguous layouts and the Satisfactory exception;
Installer rechecks the confirmed target after downloads. Payload paths are validated, scripts/setup markers excluded.
Existing INI files are preserved. Recommendations do not set tuning values.

Mutations use a per-game lock, ownership preflight and snapshots. File replacements are staged beside the
file and atomically persisted, rather than truncating existing files or writing through hard links.
The v2 manifest records installed hashes and original backups. Rollback restores safe snapshots;
concurrent changes are preserved with an explicit recovery error. Only owned v2 files authorize update/removal.
Legacy/missing/incomplete/foreign manifests cannot authorize guessed cleanup.

## Hardware and optional community runtime (2026.10.0)

See [hardware/runtime guide](hardware-runtime.md) for startup collection, explicit rendering GPU selection,
consent, pinned hashes, provenance, update and uninstall behavior. Official extraction is never patched in place.
Community DLLs use the same transactional ownership rules. Automatic updates refuse a community install
instead of silently replacing the selected runtime; use a manual update with a fresh choice.

## Discovery, artwork and privacy

See [store formats](store-formats.md), [cover art](cover-art.md) and [troubleshooting](TROUBLESHOOTING.md).
Discovery is best effort and does not establish compatibility. Hardware stays local; reports require preview
and explicit file save. No new telemetry/upload is added. The historical Python manifest fixture is retained
only to test safe preservation of old installations.

## Development and release

See [development guidance](../CLAUDE.md) and [release procedure](../RELEASE.md).
Current CI is ci-rust.yml; release-rust.yml builds authorized CalVer tags. Historical release records remain historical.
