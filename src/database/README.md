# Local database

This folder owns the local SQLite schema used by OpenList Explorer.

Tables follow the V1.0 PRD:

- `settings`
- `favorites`
- `history`
- `tasks`

Token values must not be stored here. Production token storage belongs in the
platform credential adapter: Windows Credential Manager, macOS Keychain, or
Linux Secret Service.
