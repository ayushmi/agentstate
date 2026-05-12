AgentState Gateway (Hosted Playground + Billing)

Overview
- Issues capability tokens per user/namespace using the server’s HMAC format.
- Proxies REST calls to the AgentState server and meters usage per request.
- Integrates with Stripe Billing for free tier + pay-as-you-go (optional for OSS).

Endpoints
- POST /api/token: Returns a short‑lived capability token and namespace.
- ANY /v1/*: Reverse proxy to AgentState; injects Authorization and meters usage.
- POST /webhook: Stripe webhooks (optional), usage reconciliation.

Quick Start (Dev)
1. set env:
   - CAP_KEY_ACTIVE_ID=active
   - CAP_KEY_ACTIVE=dev-secret
   - UPSTREAM_BASE=http://localhost:8080
   - FREE_MAX_QPS=5
   - FREE_MAX_BYTES=262144
2. run: npm install && npm run dev
3. open docs/playground and set localStorage as.gateway to http://localhost:8787

Notes
- SSE cannot carry Authorization headers; for watch, the proxy endpoint should be used (e.g., /v1/:ns/watch via this gateway).
- For production, back usage counters by Redis/Postgres and flush usage to Stripe metered prices using Usage Records.

