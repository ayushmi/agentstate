# Multi-Agent Board Demo (MVP)

Components:
- Planner (writer): creates tasks (type="task") with tags and body.
- Workers (2+): poll or watch, acquire lease, process, write results with Idempotency-Key.
- gRPC watcher: see `clients/python/watch_client.py` or `clients/node/watch-client`.

Run locally:
1) Start server: `DATA_DIR=./data cargo run -p agentstate-server`
2) In one terminal: `python planner.py`
3) In two terminals: `python worker.py --owner w1` and `python worker.py --owner w2`
4) Optional: gRPC watch tail: `python ../../clients/python/watch_client.py --ns acme`

Crash/restart test: kill server and workers; restart; workers should resume without duplicates.
