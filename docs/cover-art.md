# Cover artwork backend

`opticore::cover_art::CoverCache` is the new per-install artwork layer. It does not
change `images::ImageCache` or the `ArtRequest` contract used by existing callers.
The UI must add `pub mod cover_art;` to `opticore/src/lib.rs` and perform `fetch`
and `refresh` on a worker thread because disk decoding and network I/O block.

## API and precedence

- `CoverCache::new(cache_dir: &Path)` uses local Steam artwork and the bounded,
  no-redirect HTTP client. `with_sources` injects local roots and a fetcher for tests.
- `fetch(&Game) -> Option<PathBuf>` returns a selected image or saved resolution
  state without network on every render. A first miss performs one lookup.
- `source(&Game) -> Option<String>` reports `user override`, `steamgriddb`,
  `legacy cache`, `steam local`, `store supplied`, `steam store`, or `steam header`.
- `set_override(&Game, &Path) -> Result<PathBuf, String>` validates/copies a user file.
  `clear_override(&Game) -> Result<(), String>` clears the selection, not its file.
- `refresh(&Game) -> Option<PathBuf>` explicitly retries automatic sources. It
  never replaces a selected user or SteamGridDB image.
- `clear_automatic_cache() -> Result<(), String>` removes only known generated
  automatic v2 files. It leaves legacy root files and any v2 directory with
  selected-art history untouched. The settings Clear cache action should use
  this method instead of recursively removing `game_images`.
  Clearing rejects symlinks and Windows reparse points (including junctions).
  Image filenames must also be referenced by a recognized automatic state;
  unknown or unowned files cause the directory to be preserved.

The v2 key hashes platform, observed store identity, and normalized install path.
Operations serialize per game, so a network request does not block artwork or
user selections for another game. Cache clearing acquires each game lock in turn.
The directory contains append-only, versioned JPEGs and state records. Writes
use a bounded image decoder, temporary file, sync, and rename; refresh does not
purge caches. Existing `appid_<id>` and title-keyed files remain untouched and
can be shown as read-only legacy fallback. A legacy landscape is offered an
upgrade to an identified Steam portrait once, then a saved result/miss prevents
repeat network traffic. Legacy title files lack store/path provenance; their
original ambiguity cannot be repaired by the new key. The new cache never writes
to those names.

Automatic lookup is local Steam portrait, an allowlisted HTTPS store-supplied
URL, then Steam's portrait for an appid confirmed by both `steam_appid` and the
observed Steam `StoreIdentity`; Steam's header and guarded appdetails
header/capsule URLs follow. Xbox in-install logos and the executable icon remain
offline fallbacks. No fuzzy title
lookup is used to infer a Steam appid. HTTPS downloads reject redirects, and
untrusted store URLs are restricted to known store artwork domains. Failures
return a miss or the old cache; no API token is involved.

## Optional SteamGridDB flow

Only UI actions should construct `SteamGridDb::new(user_token)`, call
`search_game(&Game) -> Result<Vec<GridGame>, String>` (or
`search(title) -> Result<Vec<SearchGame>, String>`), present candidates for explicit
identity confirmation, call `grids(game_id) -> Result<Vec<GridImage>, String>`,
present the image choices, then call `select(&CoverCache, &Game, &GridImage)
-> io::Result<PathBuf>` for the chosen grid. `CoverCache::fetch` and `refresh`
never call SteamGridDB. The token is kept in memory, used only in the Bearer
header to the exact SteamGridDB API host, and is never logged or sent with image
downloads. Grid downloads are limited to the provider's documented S3 grid path
and the exact `cdn2.steamgriddb.com/grid/` path (thumbnails use `/thumb/`),
no redirects, and bounded before decoding. Callers must not auto-select the first
search or grid result.

`search_game` first uses `/games/steam/{appid}` when the Steam appid and observed
Steam store identity agree. Steam is the only supported exact external-ID mapping.
An unavailable mapping falls back to title candidates requiring confirmation;
other stores use that same confirmation flow without guessed external-ID mappings.
The official [API wrapper's endpoint and schema declarations](https://github.com/SteamGridDB/node-steamgriddb/blob/master/src/index.ts)
confirm this endpoint and its single-game `success`/`data` envelope, as well as
the search and grid endpoints. This method never runs during automatic cover fetch.

Schema evidence checked 2026-10-02: the [SteamGridDB API v2 page](https://www.steamgriddb.com/api/v2)
and [SteamGridDB's official Node wrapper documentation](https://github.com/SteamGridDB/node-steamgriddb)
show the API base URL, user API key, search candidates with `id`/`name`, grids
by game ID, and image `id`/`url`/`thumb` with S3 grid URLs. No code was copied.
Current CDN evidence: [SteamGridDB collection 14488](https://www.steamgriddb.com/collection/14488)
contains `https://cdn2.steamgriddb.com/grid/aa6488ad256e6d0242bc786fcc377e64.png`;
the [game page](https://www.steamgriddb.com/game/2254) also serves a `/thumb/` asset
on that exact host. Other CDN hostnames are not inferred or allowed.
