# v0.6.0 – 2026-08-04

## New features
- **Collection context menus** — Right-click on collections, folders, and requests for quick access to Rename, Delete, Move, and Export actions.
- **OpenAPI export** — Export any collection as an OpenAPI 3.0 JSON specification.
- **Run Collection** — Execute all requests in a collection sequentially from the context menu.
- **Collection variables** — Define per-collection variables with a key-value editor, accessible from the context menu.
- **History context menu** — 3-dot menu on history entries with View Response and Delete options.
- **History URL truncation** — Long URLs display with `...` truncation for cleaner layout.

## Improvements
- **"New" button styling** — Blue iced primary button for better visibility in collections.
- **Panel backgrounds** — History and collections panels now use the iced dark theme background for visual consistency.
- **Code quality** — Removed custom button/input style closures in history and collections views.

## Bug fixes
- **Keyring migration** — Fixed missing argument in token migration function.
- **FTS5 search injection** — Sanitized search queries to prevent FTS5 syntax errors.
- **Request send performance** — Lightweight `RequestSnapshot` replaces full view clone for sending requests.

---

# v0.5.0 – 2026-07-28

## New features
- **Mock Server** — Basic mock server with configurable endpoints, response codes, delays, and bodies.
- **GraphQL subscriptions** — WebSocket-based subscription support.
- **WebSocket message history** — Copy and re-send previous messages.
- **Streaming responses** — Progressive body display with real-time updates.
- **QuickJS scripting** — Sandboxed JavaScript execution via QuickJS engine.

## Improvements
- **OAuth2** — Improved token refresh and multi-tab polling.
- **Performance** — Cookie sync optimizations and lazy tab rendering.

## Bug fixes
- HTTP/GraphQL/WebSocket stability fixes.
- Linux CI build (`muda`/`gtk-sys` conflict resolved).

---

# v0.4.0 – 2026-07-23

## New features
- **Native OS menu bar** — Full File/Edit/View/Help menus via `muda` crate (macOS menu bar, Windows Win32 menu). Keyboard shortcuts shown inline (⌘T, ⌘S, ⌘F, etc.).
- **Form URL-Encoded body type** — `application/x-www-form-urlencoded` support with key-value editor for login forms and OAuth2 token exchange.
- **WebSocket Enter-to-send** — Pressing Enter sends the message when connected.
- **Spinner on request loading** — Animated spinner during HTTP and GraphQL requests.

## Breaking changes
- **Rebrand: AstraNova → Astraio** — Database renamed from `astranova.db` to `astraio.db`. Data paths changed from `~/.astranova/` to `~/.astraio/`. Users migrating from v0.2.x should move their database file.

## Known limitations
- Native menu bar works on macOS and Windows. Linux uses in-app fallback (no SO menu bar).
- Windows menu accelerators (Ctrl+S, etc.) require `TranslateAcceleratorW` which iced doesn't expose — shortcuts work via in-app keyboard subscriptions, not via the native menu.

---

*Generated on 2026-07-23.*
