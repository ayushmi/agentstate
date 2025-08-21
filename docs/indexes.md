# Indexes & Projections (MVP)

- Tag index: exact match on `tags.*` using per-namespace inverted maps.
- JSONPath index (opt-in): equality on materialized paths (e.g., `$.status`) configured per-namespace; MVP: declare by populating values and the engine auto-indexes when present.
- Projections: `fields=[...]` in `POST /v1/{ns}/query` trims `body` to supplied top-level keys to reduce payload.

Acceptance: queries over indexed tags/paths avoid full scans when possible; projections significantly reduce response size for large documents.
