# Astraio Client

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg)]()

<img src="assets/astra-bg.png" alt="Astraio Logo" width="300">

A fast, open-source desktop API client built with **Rust** and **Iced**. Test, debug, and manage HTTP, WebSocket, and GraphQL requests with a native, cross-platform UI.

[Website](https://astraio-client.vercel.app/) | [Download](https://astraio-client.vercel.app/) | [Report Bug](https://github.com/vant3x/astraio-client-rust/issues)

---

## Features

### HTTP Client

- **All HTTP methods** — GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
- **Request builder** — URL, headers, query params, body (JSON, XML, HTML, Text)
- **Multipart / file uploads** — Text fields and file attachments
- **Binary response detection** — Auto-decodes base64, image preview
- **Response viewer** — Formatted JSON, headers, timeline, status codes
- **Search in responses** — Find text with match navigation
- **Download responses** — Save response body to file
- **Code snippets** — Generate cURL, Python, JavaScript, Rust, Go
- **Request cancellation** — Abort in-flight requests

### Authentication

- **Bearer Token**
- **Basic Auth**
- **API Key** (header or query parameter)
- **Digest Auth** — Full RFC 2617 implementation
- **OAuth 2.0** — Authorization Code, PKCE, Client Credentials, Device Code
- **OS Keychain integration** — Tokens stored securely via `keyring`

### WebSocket Client

- Connect to any WebSocket endpoint
- Send text, binary, and ping/pong messages
- TLS with custom CA certificates and client certs (mTLS)
- Auto-reconnection settings
- Message history with hex/binary viewer
- Connection statistics (duration, bytes, message count)

### GraphQL

- Query and mutation support
- Variable editor with JSON validation
- Schema introspection (coming soon)
- Response viewer with error paths and locations

### Collections & Organization

- **Collections** — Group requests into named collections
- **Folders** — Nested folder structure
- **Save / load requests** — Persist request configs to SQLite
- **Drag-and-drop reorder** — Sort collections, folders, and requests
- **Rename inline** — Double-click to rename

### Environments

- **Environment variables** — Define key-value pairs per environment
- **Variable interpolation** — Use `{{variable}}` in URL, headers, and body
- **Default endpoint** — Set a base URL per environment
- **Export `.env`** — Save variables as `.env` files
- **Secret variables** — Mark variables as sensitive (stored in OS keychain)

### Import / Export

- **cURL import** — Paste a cURL command, auto-populates the request
- **Postman import / export** — v2.1 collection format
- **OpenAPI 3.x / Swagger 2.0 import** — Generates collections from specs
- **History export** — JSON or CSV

### Request Scripts

- **Pre-request scripts** — Modify requests before sending (set headers, variables, body)
- **Post-response scripts** — Assert status, extract JSON values, validate headers
- **13+ actions** — `set_variable`, `set_header`, `assert_status`, `extract_json`, `extract_header`, `log`, and more
- **Variable chaining** — Extract a value from one response, use it in the next request

### Developer Experience

- **Native OS menu bar** — File, Edit, View, Help menus (macOS menu bar, Windows native menu)
- **Dark / light theme** — Toggle with `Ctrl+D`
- **Keyboard shortcuts** — `Ctrl+N` (new tab), `Ctrl+W` (close tab), `Ctrl+1-5` (switch tabs)
- **Tab management** — Multiple request tabs with URL preview
- **Request settings** — Timeout, retry with backoff, redirect policy, proxy config
- **TLS / mTLS** — Custom CA certs, client certificates, skip SSL verification
- **Toast notifications** — Non-intrusive success/error feedback
- **Request history** — Auto-saved with search, filter by method, re-send with one click

---

## Screenshots

> _Screenshots coming soon. Run `cargo run` to see the app in action._

---

## Architecture

```
src/
├── main.rs                  # Entry point
├── ui/                      # Iced GUI layer
│   ├── app.rs               # Main application state & message loop
│   ├── views/               # View modules per tab/panel
│   │   ├── http_request_view/
│   │   ├── collection_view.rs
│   │   ├── environment_manager.rs
│   │   ├── history_view.rs
│   │   ├── websocket_view.rs
│   │   └── graphql_view.rs
│   ├── handlers/            # Message handlers per domain
│   ├── components/          # Reusable UI components
│   ├── menu.rs              # Native OS menu bar (muda)
│   ├── theme.rs             # Colors and theming
│   └── toast.rs             # Toast notification system
├── http_client/             # HTTP request/response layer
│   ├── client.rs            # reqwest client builder, redirect handling, digest auth
│   ├── request.rs           # HttpRequest model
│   ├── response.rs          # HttpResponse model
│   ├── config.rs            # RequestConfig (timeout, retry, proxy, TLS)
│   └── snippets.rs          # Code snippet generation
├── protocols/               # Protocol implementations
│   ├── websocket.rs         # WebSocket client (tokio-tungstenite)
│   ├── graphql.rs           # GraphQL request/response models
│   ├── graphql_schema.rs    # Schema introspection
│   └── scripts.rs           # Pre/post-request script engine
├── data/                    # Data models
│   ├── auth.rs              # Auth types (Bearer, Basic, OAuth2, etc.)
│   ├── auth_input.rs        # Auth input handling
│   └── oauth2.rs            # OAuth2 flows, PKCE, Device Code
├── persistence/             # SQLite storage
│   └── database.rs          # Schema, CRUD for collections, environments, history
├── services/                # Business logic layer
│   ├── collection_service.rs
│   ├── environment_service.rs
│   ├── history_service.rs
│   ├── secret_store.rs      # OS keychain integration
│   └── request_restoration.rs
├── import/                  # Import formats
│   ├── curl.rs              # cURL command parser
│   └── postman.rs           # Postman collection parser
├── export/                  # Export formats
│   └── postman.rs           # Postman collection export
├── openapi/                 # OpenAPI/Swagger import
│   ├── parser.rs
│   ├── models.rs
│   └── collection_generator.rs
├── error.rs                 # AppError enum
└── utils.rs                 # Timestamps and helpers
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| **Language** | Rust (edition 2021) |
| **GUI** | [Iced](https://iced.rs/) 0.14 (Elm architecture) |
| **HTTP** | [reqwest](https://docs.rs/reqwest) 0.12 (rustls-tls) |
| **WebSocket** | [tokio-tungstenite](https://docs.rs/tokio-tungstenite) 0.24 |
| **Database** | [rusqlite](https://docs.rs/rusqlite) 0.31 (SQLite, WAL mode) |
| **Async runtime** | [tokio](https://docs.rs/tokio) 1.38 |
| **Secrets** | [keyring](https://docs.rs/keyring) 3 (macOS Keychain, Windows Credential Manager, Linux Secret Service) |
| **Native menus** | [muda](https://github.com/tauri-apps/muda) 0.17 |
| **TLS** | native-tls / rustls |

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.75+
- Platform dependencies (Linux only):
  ```bash
  # Ubuntu/Debian
  sudo apt install libxkbcommon0 libwayland-client0 libxrandr2 libxcursor1 libxi6 libxinerama1
  ```

### Install & Run

```bash
git clone https://github.com/vant3x/astraio-client-rust.git
cd astraio-client-rust
cargo run --release
```

First build will take a few minutes. Subsequent builds are fast.

### Run Tests

```bash
cargo test
```

### Build Release

```bash
cargo build --release
# Binary: target/release/astraio-client
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` / `Ctrl+T` | New request tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+D` | Toggle dark/light theme |
| `Ctrl+E` | Toggle environment manager |
| `Ctrl+F` | Toggle response search |
| `Ctrl+←` / `Ctrl+→` | Navigate tabs |
| `Ctrl+1` through `Ctrl+5` | Switch to tab by number |

---

## Platform Notes

### macOS

If macOS blocks the app (unverified developer), run:

```bash
xattr -cr /Applications/Astraio.app
```

### Windows

The app is built as a windowed application (no console window).

### Linux

Ensure the required system libraries are installed (see Prerequisites).

---

## Project Status

**Version:** 0.6.0

| Feature | Status |
|---------|--------|
| HTTP requests | Stable |
| Authentication (all types) | Stable |
| Collections & folders | Stable |
| Environments & variables | Stable |
| Request history | Stable |
| cURL import | Stable |
| Postman import/export | Stable |
| OpenAPI import | Stable |
| WebSocket client | Stable |
| GraphQL | Stable |
| Request scripts | Stable |
| TLS / mTLS | Stable |
| Proxy support | Stable |
| OS Keychain secrets | Stable |
| Code snippets | Stable |
| Image preview | Stable |
| CLI mode | Beta |
| GraphQL schema viewer | Planned |
| Request drag-and-drop reorder | Planned |
| Mock server | Planned |

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes (`git commit -am 'Add my feature'`)
4. Push to the branch (`git push origin feature/my-feature`)
5. Open a Pull Request

Please run `cargo test` and `cargo clippy` before submitting.

---

## License

This project is **dual-licensed**:

- **MIT** — Free for individual developers and small teams. See [LICENSE](LICENSE).
- **Commercial** — Required for enterprise features like team collections, cloud sync, and priority support. See [LICENSING.md](LICENSING.md).

---

## Acknowledgments

- [Iced](https://iced.rs/) — Beautiful cross-platform GUI framework for Rust
- [reqwest](https://github.com/seanmonstar/reqwest) — Ergonomic HTTP client
- [Postman](https://www.postman.com/) — Inspiration for the API client experience
- [Bruno](https://www.usebruno.com/) — Inspiration for the open-source approach
