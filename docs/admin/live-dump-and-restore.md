# Live Dump & Restore Validation

## Live dump (dev only)

Expose dev-only endpoint (behind `DEV_MODE=1`) to stream JSONL of current live objects—used for integrity comparisons.

```
GET /admin/dump?ns=<ns>&fields=body,tags,ts
Content-Type: application/x-ndjson
```

Each line: `{"id":"...","ns":"...","body":{...},"tags":{...},"ts":"...","commit_seq":123}`

Security: Require `verbs=["admin"]` and `ns="admin://global"`. Never enable in prod.

## Restore validation flow

1) Take snapshot:

```
POST /admin/snapshot  -> {"snapshot_id":"snap-…","last_seq":N}
```

2) Trim old WAL:

```
POST /admin/trim-wal?snapshot_id=snap-…
```

3) Offline restore:

```
agentstate restore --snapshot /data/snapshots/snap-… \
                   --wal /data/wal --out /tmp/report.json \
                   --dump /tmp/restore.jsonl
```

4) Live dump (dev env only) and diff:

```
curl -H "Authorization: Bearer <admin-cap>" \
  "http://localhost:8080/admin/dump?ns=agent://acme" \
  -o /tmp/live.jsonl

# lightweight diff idea: normalize & compare id->hash(body,tags)
```

Success criteria: `report.json` shows `crc_ok=true`, `index_consistent=true`, and live-vs-restore hashes match for the same `last_seq`.

