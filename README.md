# OpenList Explorer

OpenList Explorer is a desktop storage resource manager built on top of the
OpenList REST API. OpenList owns storage protocols and drivers; Explorer owns
the user experience.

## Stack

- Tauri 2.x
- Vue 3
- TypeScript
- Pinia
- Vue Router
- Naive UI
- VueUse
- Axios
- SQLite schema with Drizzle ORM

## MVP scope

- Storage switching
- File browsing
- Upload
- Download
- Delete
- Rename
- Search
- Raw link copy
- Favorites
- History
- Settings

## Development

```bash
npm install
npm run dev
```

For the desktop shell:

```bash
npm run tauri:dev
```

Tokens must not be stored in SQLite or browser local storage. The current MVP
keeps tokens in memory until the platform credential adapter is implemented.
