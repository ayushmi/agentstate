# Quickstart

- Prereqs: Rust (stable), Docker (optional), Node 18+ (optional), Python 3.9+ (optional)

## Run the server locally

- `cargo run -p agentstate-server`
- Health check: `curl localhost:8080/health`

## Basic HTTP API

- Put:

```
curl -sX POST localhost:8080/v1/acme/objects \
  -H 'content-type: application/json' \
  -d '{"type":"note","body":{"text":"hello"},"tags":{"topic":"demo"}}'
```

- Get:

```
curl -s localhost:8080/v1/acme/objects/<id>
```

- Query:

```
curl -sX POST localhost:8080/v1/acme/query \
  -H 'content-type: application/json' \
  -d '{"tag_filter":{"topic":"demo"}}'
```

- Watch (SSE):

```
curl -N localhost:8080/v1/acme/watch  # events include commit resume token
```

## Leases and Idempotency

- Acquire a lease:

```
curl -sX POST localhost:8080/v1/acme/lease/acquire \
  -H 'content-type: application/json' \
  -d '{"key":"task-1","owner":"worker-a","ttl":30}'
```

- Idempotent put:

```
curl -sX POST localhost:8080/v1/acme/objects \
  -H 'content-type: application/json' -H 'Idempotency-Key: abc123' \
  -d '{"type":"note","body":{"text":"hello"}}'
```

## SDKs (MVP)

- Python: see `sdk-py/README.md`
- TypeScript: see `sdk-ts/README.md`
