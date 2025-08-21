# Compatibility Contract (v0.1.x)

This document defines the surface areas frozen for v0.1.x and the semver/deprecation policy.

## Frozen for v0.1.x

- APIs: HTTP endpoints/params, gRPC services/messages.
- Watch semantics: commit_seq required, resume from from_commit (inclusive), overflow behavior as documented in docs/watch.md.
- Capability tokens: required claims (ns, verbs, exp, region, limits), dual-key rotation acceptance (active/next), error codes (401,403,413,429,451).
- Operational knobs (env): WAL_*, WATCH_*, TLS_*, CAP_*, REGION
- Metrics: exported names/labels are public APIs (see dashboard and server metrics module).
- On-disk layout: data dir with manifest.json, wal/ segments, snapshots/ (forward-compatible manifest fields; snapshot_bookmark).

## Semver policy

- Major (X): may break wire formats or watch semantics.
- Minor (Y): new endpoints/metrics/envs only; no breaks.
- Patch (Z): fixes & perf only.

## Deprecation

- Changes to any frozen surface require deprecation notes in release docs.
- Keep deprecated fields/endpoints for 2 minor releases before removal.

