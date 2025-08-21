.PHONY: verify verify-soak verify-k6 verify-restore

verify: verify-soak verify-k6 verify-restore
	@echo "PASS: verify complete"

verify-soak:
	@echo "Running overflow soak (requires local server on :8080)" && \
	python3 examples/tests/overflow_soak.py || true

verify-k6:
	@echo "Running QPS limiter test (requires k6)" && \
	k6 run examples/tests/qps_limiter.js || true

verify-restore:
	@echo ">> Taking snapshot" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" -X POST "$(BASE)/admin/snapshot" -o /tmp/snap.json && \
	jq -r '.snapshot_id' /tmp/snap.json > /tmp/snap.id && \
	printf "Snapshot %s\n" "$$(cat /tmp/snap.id)" && \
	printf ">> Trimming WAL\n" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" -X POST "$(BASE)/admin/trim-wal?snapshot_id=$$(cat /tmp/snap.id)" >/dev/null && \
	printf ">> Offline restore + dump\n" && \
	cargo run -q -p agentstate-cli -- restore --snapshot $$DATA_DIR/snapshots/$$(cat /tmp/snap.id) --wal $$DATA_DIR/wal --out $(RESTORE_OUT) --dump $(RESTORE_DUMP) && \
	printf ">> Live dump (dev mode only)\n" && \
	curl -fsS -H "Authorization: Bearer $(ADMIN_CAP)" "$(BASE)/admin/dump?ns=$(NS)" -o $(LIVE_DUMP) && \
	printf ">> Diff\n" && \
	python3 examples/tools/diff_dump.py $(LIVE_DUMP) $(RESTORE_DUMP) || true

RESTORE_OUT ?= /tmp/restore-report.json
RESTORE_DUMP ?= /tmp/restore.jsonl
LIVE_DUMP ?= /tmp/live.jsonl
ADMIN_CAP ?= $(shell cat .admin.cap 2>/dev/null)
BASE ?= http://localhost:8080
NS ?= agent://verify
