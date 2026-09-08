# ADR 0001 — Single Rust process for collectors + API

- Status: accepted
- Spec: vigilon-project-spec.md §5

A Python API in front of collectors would add a process hop, a second runtime, and overhead the product positioning rejects. Axum serves REST and WebSocket from the same Tokio process that owns collectors and SQLite.

Python remains allowed only as a throwaway prototype, never in the shipped path.
