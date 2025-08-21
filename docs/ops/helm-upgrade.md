# Helm Zero-Downtime Upgrade

## Readiness gates

Pod is Ready only when:
- TLS cert/key files are readable (or TLS explicitly disabled).
- CAP_KEY_ACTIVE(_ID) present (and optionally CAP_KEY_NEXT(_ID)).
- `/health` returns 200 (server fully bound).

Example values excerpt:

```yaml
env:
  REGION: "eu-west-1"
  CAP_KEY_ACTIVE_ID: "active"
  CAP_KEY_ACTIVE: "<base64>"
  # CAP_KEY_NEXT_ID / CAP_KEY_NEXT can be set during rotation
  TLS_CERT_PATH: "/certs/tls.crt"
  TLS_KEY_PATH: "/certs/tls.key"
  # TLS_CLIENT_CA_PATH: "/certs/ca.crt"
readinessProbe:
  httpGet:
    scheme: HTTPS
    path: /health
    port: 8080
  periodSeconds: 5
  failureThreshold: 3
livenessProbe:
  httpGet:
    scheme: HTTPS
    path: /health
    port: 8080
lifecycle:
  preStop:
    exec:
      command: ["/bin/sh","-lc","sleep 3"]
strategy:
  type: RollingUpdate
  rollingUpdate:
    maxUnavailable: 0
    maxSurge: 1
securityContext:
  runAsNonRoot: true
  readOnlyRootFilesystem: true
```

## Upgrade sequence

1. Stage NEXT key envs (see key-rotation.md). `helm upgrade …` → new pods Ready; old tokens still verify.
2. Begin issuing tokens with `kid=NEXT`.
3. Promote NEXT→ACTIVE, clear NEXT envs; `helm upgrade …`.
4. Observe metrics: `watch_drops_total` steady; no spike in 401/403; dashboard healthy.
5. Rollback safety: both ACTIVE and NEXT accepted—safe to re-flip if needed.

