# Tests

- Rust: `cargo test --workspace` (unit tests in `core`, `agent`, `platform`; SQLite round-trip in `core::storage`; REST API smoke tests in `api`)
- Frontend: `cd frontend && npm test` (Vitest unit tests in `lib.test.ts`)
- Frontend build: `cd frontend && npm run build` (TypeScript check and Vite bundle)
