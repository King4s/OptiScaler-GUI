# Installed-store discovery: format contract

This is a read-only discovery interface, not an installation authority. Every
emitted entry must have an observed store identifier and an existing game
directory. Missing stores are normal; malformed or inaccessible present stores
produce nonfatal warnings. Inputs are capped and parsing never follows an
install path to write to it.

## Amazon Games

Observed on the development Windows machine at
`%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite`:
SQLite table `DbSet` has `Id`, `InstallDirectory`, `Installed`,
`ProductAsin`, and `ProductTitle` columns. The observed database had zero
installed rows, so row semantics are also corroborated by the independent
[Playnite Amazon adapter](https://github.com/JosefNemec/PlayniteExtensions/blob/master/source/Libraries/AmazonGamesLibrary/AmazonGamesLibrary.cs).
Read `Installed = 1` records only; `ProductAsin` is the store ID. Require
nonempty `ProductTitle`, `ProductAsin`, and `InstallDirectory`. Open the DB
read-only with SQLite and cap its file size and returned rows. No filename-
derived ID or fallback title is accepted.

## Ubisoft Connect

Observed `HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs` on the
development Windows machine: numeric subkeys (`5210`, `6100`, etc.) with an
`InstallDir` string. A second unredirected path is probed for other Windows
installations. The numeric key is the observed Ubisoft ID; a present install
directory's folder name supplies the display title. Non-numeric keys are
rejected. No static ID-to-title table is used.

## EA app / Origin compatibility

The EA app's own [support page](https://help.ea.com/en/help/ea/ea-app/download-install-update-ea-app/)
documents the launcher install location, **not** a stable installed-game
catalogue. A [user report](https://www.reddit.com/r/EAApp/comments/16al1rm/guide_how_to_find_installed_games_in_eaapp/)
describes per-game `HKLM\SOFTWARE\EA Games` keys and `Install Dir`; it does
**not** establish a `Product GUID` schema. A separate
[EA forum report for Titanfall 2](https://forums.ea.com/discussions/titanfall-franchise-discussion-en/titanfall-2-installed-on-steam-but-ea-wont-detect-it-being-installed/11384325/replies/11384326)
shows `DisplayName`, `Install Dir`, and `Product GUID` on a specific game key.
That supports a fallback, not universal EA-app coverage.
Probe both 32- and 64-bit registry views, plus legacy `Origin Games` keys.
Also inspect game keys below `Respawn`, `Maxis`, and `BioWare`; these publisher
locations appear in the linked Titanfall report and EA forum reports for
[The Sims 4](https://forums.ea.com/discussions/the-sims-4-technical-issues-pc-en/initilization-error-at-start-up-135dec4090f690cf00000075495f32a0/11847307/replies/11847317).
Emit only records with an existing game directory, a nonempty display title,
and an actual `Product GUID` registry value. A game-name subkey is not an ID.
For modern installs, a game's own `__Installer/installerdata.xml` contains
`contentIDs/contentID`, as independently observed by the
[Lutris EA adapter](https://github.com/lutris/lutris/blob/master/lutris/services/ea_app.py).
The `DiPManifest` root and `gameTitles/gameTitle` fields are also described in
[nicedeck's EAInstallerData format documentation](https://pkg.go.dev/github.com/mateussouzaweb/nicedeck/src/platforms/launcher#EAInstallerData).
Read this manifest from immediate children of known EA library roots and from
registry-linked custom install paths. Use the first nonempty observed content
ID as store ID; title is manifest `en_US` title, otherwise another manifest
title, otherwise folder-derived with `TitleSource::Folder`. The parser is
bounded; malformed files warn without aborting other stores. Custom roots
with neither a registry path nor a known library root remain undiscovered.
The development machine had no EA game entries, so this adapter is
synthetic-fixture tested.

## Battle.net

Observed `%PROGRAMDATA%\Battle.net\Agent\product.db` on the development
machine, with top-level protobuf wire field 1 containing ProductInstall
messages (the first observed record had `agent` fields). The field layout is
documented in the independently published
[ProductDb schema](https://github.com/bartok765/galaxy_blizzard_plugin/blob/83cfb024ebc00cbc71702f101671bea596cfc96f/src/product_db.proto):
ProductInstall field 2 is `product_code`, field 3 is settings, and settings
field 1 is `install_path`. The scanner parses only these fields with a bounded
wire decoder; it does not copy any third-party implementation. `agent` and
`bna` are launcher records and are excluded. Product code is the store ID,
while a present directory's folder name supplies the title. A stale path is
not an installed game. The cached `installed` bit is not used because the
directory is directly checked and the cached bit may be stale.

All four adapters also apply the existing game-folder heuristic and launcher
title filter before emission. The scanner never creates directories, alters
registry keys, or treats catalogue membership as permission to install.
`StoreEntry.source` is `Registry` for Ubisoft and EA registry entries,
`StoreManifest` for EA installer manifests, and `LauncherLibrary` for Amazon
and Battle.net. `title_source` is `Folder` for Ubisoft/Battle.net folder titles,
`Store` for Amazon/EA metadata titles, and `Folder` for an EA manifest without
a title.

## Integration and verification

Public entrypoint: `scan::stores::scan_installed() -> StoreScan`.
`StoreEntry` carries `name`, `path`, `platform`, `store_id`, `art_url`,
`source`, and `title_source`. Fixture entrypoints are `scan_amazon_db`,
`scan_battlenet_db`, `scan_ubisoft_records`, `scan_ea_records`,
`scan_ea_manifest`, and `scan_ea_root`. No artwork URL is invented.

Dependencies supplied by the coordinator: `rusqlite 0.40.2` with `bundled`,
and `quick-xml 0.42.0`. SQLite is opened read-only with no lock wait. Binary
databases are capped at 16 MiB; EA XML at 2 MiB, 50,000 events, 64 nesting
levels, and 16 KiB metadata text. Directory and registry scans cap records at
4,096; each result caps warnings at 64. XML accepts the exact metadata element
paths, decodes entities, and rejects DTDs. The protobuf decoder has no recursive
schema traversal beyond the two known nested levels.

Focused verification: `cargo test -p opticore --test store_discovery`:
11 passed, one local smoke test ignored by default. Running that smoke test
explicitly on the development machine found Amazon 0, Ubisoft 2, EA 0,
Battle.net 0, with 3 nonfatal warnings. Only platform counts were printed;
no game files or launcher metadata were changed. This is positive local
discovery evidence for Ubisoft, not a live installed-game validation of the
other three adapters. All fixtures and implementation code are original.
