# Changelog

## v0.6.0 (2026-08-04)

### Added
- **Collections — Context menus** — Right-click context menus on collections, folders, and requests with Rename, Delete, Move, Export actions.
- **Collections — OpenAPI export** — Export collections as OpenAPI 3.0 JSON specifications.
- **Collections — Run Collection** — Run all requests in a collection sequentially.
- **Collections — Variables editor** — Per-collection variables with key-value editor.
- **History — Context menu** — 3-dot context menu on history entries with View Response and Delete actions (replaces inline buttons).
- **History — URL truncation** — Long URLs are visually truncated with `...` while remaining selectable.
- **History — Method/status badges** — Colored method and status code badges on each entry.

### Changed
- **Collections — "New" button** — Moved to its own row using iced primary button style (blue) for better visibility.
- **Collections — Import/export buttons** — Postman, HAR, and OpenAPI buttons moved to a single row with consistent default styling.
- **History/Collections panel backgrounds** — Updated to use `iced::Theme::Dark.palette().background` for consistency with the rest of the app.
- **History — Response panel** — Close button uses default iced button style instead of custom styling.
- **Code quality** — Removed all custom `button::Style` and `text_input::Style` closures from history and collection views in favor of iced defaults.

### Fixed
- **Keyring migration** — Fixed `migrate_plaintext_tokens_to_keyring` call with missing `SecretStore::new()` argument.
- **FTS5 injection** — Added `sanitize_fts5_query()` to wrap search queries in escaped double quotes, preventing FTS5 syntax errors.
- **Request cloning overhead** — Replaced full `HttpRequestView` clone with lightweight `RequestSnapshot` struct (12 fields vs 50+) for send operations.

### Tests
- 426 passing, 0 clippy warnings.

## v0.5.0 (2026-07-28)

### Added
- **Mock Server** — Basic mock server with configurable endpoints, response codes, delays, and bodies.
- **GraphQL subscriptions** — WebSocket-based GraphQL subscription support.
- **WebSocket copy/re-send** — Copy messages and re-send from history.
- **Streaming HTTP responses** — Progressive body display with real-time updates.
- **QuickJS scripting engine** — Replaced native script engine with QuickJS for safer, sandboxed script execution.
- **Collection loading in GraphQL** — Load saved requests from collections into GraphQL view.

### Changed
- **Script engine** — Migrated from custom native engine to QuickJS (`rquickjs` crate) for better compatibility and security.
- **OAuth2 flow** — Improved token refresh and multi-tab polling support.

### Fixed
- **Cookie sync and lazy tab rendering** — Performance optimizations.
- **HTTP/GraphQL/WebSocket bugs** — Various stability fixes across all protocol handlers.
- **Clippy warnings** — Fixed needless borrow warnings in `rfd::FileDialog` calls.
- **Linux CI build** — Resolved `muda`/`gtk-sys` feature conflict.

### Tests
- 410+ passing.

## v0.4.0 (2026-07-23)

### Added
- **Native OS menu bar** — File, Edit, View, Help menus via `muda` (macOS menu bar, Windows Win32 menu).
  - File: New Tab, Open Collection, Save, Import (cURL/Postman/OpenAPI), Export (Postman/HAR), Quit
  - Edit: Undo, Redo, Cut, Copy, Paste, Select All, Find
  - View: Toggle Sidebar, Toggle History, Toggle Collections, Toggle Dark Mode, New Window
  - Help: About Astraio
- **Form URL-Encoded body type** — `application/x-www-form-urlencoded` support with key-value editor.
- **WebSocket Enter-to-send** — Pressing Enter sends the message when connected.
- **Spinner on request loading** — Animated spinner (`iced_aw::Spinner`) during HTTP and GraphQL requests.
- Confirmation dialogs for history delete/clear.
- SSL verification disabled warning banner.
- OAuth2 token data sanitized before SQLite storage.
- WAL mode + foreign keys enabled for SQLite.

### Changed
- **Rebrand: AstraNova → Astraio** — Renamed across the entire codebase: struct names, database name, paths (`~/.astraio/`), CLI, HAR export creator, OAuth2 HTML titles, WiX installer.
- **Views refactor** — Split monolithic `views.rs` (1900 lines) into 10 focused modules (~200-420 lines each): `body_tab`, `auth_tab`, `settings_tab`, `cookies_tab`, `scripts_tab`, `response_area`, `snippets_panel`, `helpers`.
- **Toolbar spacing fix** — Request bar row kept inline in `view()` with exact `.spacing(10).padding(iced::Padding::from([16, 10]))` to preserve correct vertical spacing between toolbar and tabs.

### Fixed
- **Script delay blocking UI** — `ScriptAction::Delay` logs a warning instead of blocking the UI thread.
- **Proxy auth** — Uses `Proxy::basic_auth()` instead of embedding credentials in URL string.
- **OAuth2 functions** — Shared `reqwest::Client` instead of creating new client per request.
- **Menu timing on macOS** — Menu attached via `WindowOpened` subscription (after event loop starts), not during `AstraioApp::new()`.

### Tests
- 388 passing, 0 clippy warnings.
