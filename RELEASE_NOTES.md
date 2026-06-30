# v0.2.2-beta.2 – 2026‑06‑30

## New features
- **OpenAPI import UI**
  - Added an **“Import OpenAPI”** button next to the existing “Import” button in the Collections header.
  - File‑picker accepts `.json`, `.yaml`, `.yml`.
  - Shows a toast with success/failure after import.
  - Generates collections, folders, and stores them in SQLite.
- **Export .env**
  - New **“Export .env”** button in the Environment Manager.
  - Saves all environment variables (`key=value` format) to a file named `{environment}.env` via a standard file‑save dialog.
- **GraphQL refinements (v0.2.2-beta.2)**
  - Added `Message::ImportOpenApi` / `ImportOpenApiData` to the collection UI.
  - Integrated full OpenAPI import flow (parse, generate collection, persist to SQLite).
  - Removed the unused `GraphQLResponseResult` alias that was causing CI warnings.
  - Minor UI polish and consistency fixes.
- **Version bump**
  - Updated `Cargo.toml` to `0.2.2-beta.2`.
- **Test suite**
  - All **243 tests** continue to pass.

## Breaking changes
- None (only additions and internal refactors).

## Known‑TODO
- [ ] Schema viewer for imported models.  
- [ ] Server‑variable auto‑completion & security‑scheme auto‑applied.  
- [ ] Response validation and mock‑server generation.  
- [ ] Batch import of selective endpoints.

## Full changelog (since v0.2.1-beta.1)

| Area                     | Details                                                                 |
|--------------------------|-------------------------------------------------------------------------|
| **OpenAPI**              | Added import button, file picker, collection generation, $ref handling. |
| **Environments**         | Added “Export .env” button and functionality.                           |
| **GraphQL**              | New Message variants, dead‑code cleanup, CI warning removal.            |
| **CI / Lint**            | Removed unused alias `GraphQLResponseResult`; all warnings cleared.      |
| **Tests**                | 243 tests passing.                                                      |
| **Dependencies**         | Bumped minor version to `0.2.2-beta.2`; no new runtime dependencies.    |

---  
*Generated on 2026‑06‑30.*