# AgentState Deployment Guide

Quick deployment guide to get AgentState running in production environments.

## Prerequisites

- Docker & Docker Compose (for Docker deployment)
- Kubernetes cluster with kubectl access (for Helm deployment)
- Helm 3.x (for Kubernetes deployment)

## A. Quickstart (Docker, 10 minutes)

### 1. Generate Admin Capability Key

```bash
# Generate a base64-encoded HMAC key for admin operations
openssl rand -base64 32
# Example output: dGhpc2lzYXNhbXBsZWtleWZvcmRlbW9zdHJhdGlvbnB1cnBvc2Vz
```

### 2. Run with Docker

```bash
# Quick start with environment variables
docker run -d \
  --name agentstate \
  -p 8080:8080 \
  -p 9090:9090 \
  -e DATA_DIR=/data \
  -e REGION=us-west-2 \
  -e CAP_KEY_ACTIVE_ID=active \
  -e CAP_KEY_ACTIVE=dGhpc2lzYXNhbXBsZWtleWZvcmRlbW9zdHJhdGlvbnB1cnBvc2Vz \
  -v agentstate-data:/data \
  ghcr.io/REPLACE_ORG/agentstate-server:v0.1.0-rc.1

# Check health
curl http://localhost:8080/health
```

### 3. Using Docker Compose

```bash
# Clone and configure
git clone <repository>
cd AgentState

# Edit docker-compose.yml and replace REPLACE_BASE64_KEY with your generated key
# Then start the service
docker compose up -d

# Verify service is running
curl http://localhost:8080/health
```

### 4. Test with SDK

**Python Example:**
```python
import agentstate

client = agentstate.Client("http://localhost:8080")

# Put some state
obj = client.put("agent://demo", {
    "id": "task-1",
    "body": {"status": "running", "progress": 0.1},
    "tags": {"type": "task", "priority": "high"}
})
print(f"Created object with commit_seq: {obj.commit_seq}")

# Watch for changes
for event in client.watch("agent://demo"):
    print(f"Event: {event.type} for {event.id}")
    if event.type == "PUT":
        print(f"  Body: {event.body}")
```

**TypeScript Example:**
```typescript
import { Client } from '@agentstate/client';

const client = new Client('http://localhost:8080');

// Put some state  
const obj = await client.put('agent://demo', {
  id: 'task-1',
  body: { status: 'running', progress: 0.1 },
  tags: { type: 'task', priority: 'high' }
});
console.log(`Created object with commit_seq: ${obj.commit_seq}`);

// Watch for changes
for await (const event of client.watch('agent://demo')) {
  console.log(`Event: ${event.type} for ${event.id}`);
  if (event.type === 'PUT') {
    console.log(`  Body:`, event.body);
  }
}
```

---

## B. Kubernetes (Helm, 20 minutes)

### 1. Prerequisites Check

```bash
# Verify cluster access
kubectl cluster-info

# Verify Helm
helm version
```

### 2. Install AgentState

```bash
# Add Helm repository (once available)
helm repo add agentstate https://charts.agentstate.dev
helm repo update

# Or install from local chart
helm package deploy/helm
```

### 3. Configure Values

Create `values.yaml`:
```yaml
# values.yaml
image:
  repository: ghcr.io/REPLACE_ORG/agentstate-server
  tag: v0.1.0-rc.1
  
env:
  DATA_DIR: "/data"
  REGION: "us-west-2"
  DEV_MODE: "false"
  
# Security: Store sensitive values in Kubernetes secrets
secrets:
  CAP_KEY_ACTIVE_ID: "active"
  CAP_KEY_ACTIVE: "dGhpc2lzYXNhbXBsZWtleWZvcmRlbW9zdHJhdGlvbnB1cnBvc2Vz"

persistence:
  enabled: true
  size: 10Gi
  storageClass: "gp2"  # Adjust for your cluster

resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 512Mi

# Enable TLS
tls:
  enabled: true
  certManager: true  # Use cert-manager for TLS
  
# Monitoring  
monitoring:
  prometheus:
    scrape: true
  grafana:
    dashboard: true
```

### 4. Deploy

```bash
# Create namespace
kubectl create namespace agentstate

# Deploy with custom values
helm upgrade --install agentstate ./agentstate-*.tgz \
  -n agentstate \
  -f values.yaml

# Verify deployment
kubectl -n agentstate get pods
kubectl -n agentstate get svc
```

### 5. TLS Configuration

For production with custom TLS certificates:

```bash
# Create TLS secret
kubectl create secret tls agentstate-tls \
  --cert=path/to/cert.pem \
  --key=path/to/key.pem \
  -n agentstate

# Update values.yaml
tls:
  enabled: true
  secretName: agentstate-tls
```

### 6. Access Service

```bash
# Port-forward for testing
kubectl -n agentstate port-forward svc/agentstate 8080:8080

# Or configure Ingress/LoadBalancer in values.yaml
ingress:
  enabled: true
  hostname: agentstate.example.com
  tls: true
```

### 7. Import Grafana Dashboard

```bash
# Dashboard JSON is available at:
kubectl -n agentstate get configmap agentstate-grafana-dashboard -o yaml

# Or from repository
curl -O https://raw.githubusercontent.com/REPLACE_ORG/agentstate/main/deploy/grafana/agentstate-dashboard.json

# Import to your Grafana instance
```

---

## C. Security (Caps & Regions)

### Admin Capability Keys

AgentState uses HMAC-based capability keys for authentication:

```bash
# Generate production keys
ACTIVE_KEY=$(openssl rand -base64 32)
NEXT_KEY=$(openssl rand -base64 32)   # For rotation

# Set in environment
export CAP_KEY_ACTIVE_ID="prod-2024-01"
export CAP_KEY_ACTIVE="$ACTIVE_KEY"
export CAP_KEY_NEXT_ID="prod-2024-02" 
export CAP_KEY_NEXT="$NEXT_KEY"
```

**Example capability payload:**
```json
{
  "ns": "agent://production",
  "ops": ["PUT", "GET", "DELETE", "WATCH"],
  "exp": 1735689600,
  "iat": 1704067200,
  "regions": ["us-west-2", "us-east-1"]
}
```

### Key Rotation Process

```bash
# Step 1: Deploy with NEXT key alongside ACTIVE
# Step 2: Update clients to use NEXT key  
# Step 3: Promote NEXT to ACTIVE
export CAP_KEY_ACTIVE_ID="$CAP_KEY_NEXT_ID"
export CAP_KEY_ACTIVE="$CAP_KEY_NEXT"

# Step 4: Generate new NEXT key
export CAP_KEY_NEXT_ID="prod-2024-03"
export CAP_KEY_NEXT=$(openssl rand -base64 32)
```

### Region Pinning

Expected error responses:
- **451 Unavailable For Legal Reasons**: Region not allowed
- **413 Request Entity Too Large**: Payload size exceeded
- **429 Too Many Requests**: Rate limit exceeded

---

## D. Point-in-Time Restore (PITR)

### 1. Create Snapshot

```bash
# Create named snapshot
curl -X POST -H "Authorization: Bearer $ADMIN_CAP" \
  http://localhost:8080/admin/snapshot

# Response: {"snapshot_id": "snap-01HQXVGZM8...", "commit_seq": 12345}
```

### 2. Trim WAL

```bash
# Trim WAL up to snapshot
curl -X POST -H "Authorization: Bearer $ADMIN_CAP" \
  "http://localhost:8080/admin/trim-wal?snapshot_id=snap-01HQXVGZM8..."
```

### 3. Restore Process

```bash
# Stop current instance
docker compose down
# OR: kubectl scale deployment agentstate --replicas=0

# Restore using CLI
docker run --rm -v agentstate-data:/data \
  ghcr.io/REPLACE_ORG/agentstate-cli:v0.1.0-rc.1 \
  restore --data-dir /data --snapshot-id snap-01HQXVGZM8...

# Verify restoration
docker run --rm -v agentstate-data:/data \
  ghcr.io/REPLACE_ORG/agentstate-cli:v0.1.0-rc.1 \
  admin dump --data-dir /data | head -10
```

### Expected Success Indicators

- Snapshot created with increasing commit_seq  
- WAL trimmed, old segments removed
- Restore completes without CRC errors
- Admin dump shows consistent state

---

## E. Troubleshooting

### Watch Overflow/Resume

**Symptoms:** Clients miss events, high backlog metrics

```bash
# Check watch metrics
curl http://localhost:8080/metrics | grep watch_

# Expected behavior
# watch_clients_total - number of active clients
# watch_drops_total - events dropped due to overflow  
# watch_backlog_events - buffered events per client
# watch_emit_lag_seconds - p95 latency for event delivery
```

**Tuning:** Increase buffer sizes in deployment:
```yaml
env:
  WATCH_BUFFER_SIZE: "10000"    # events per client
  WATCH_BUFFER_BYTES: "10485760" # 10MB per client  
```

### QPS Limiter Behavior

**429 Too Many Requests responses:**
- Per-namespace rate limiting active
- Client should implement exponential backoff
- Check metrics: `rate_limit_exceeded_total`

### TLS/mTLS Misconfiguration

**Common issues:**
```bash
# Certificate verification failed  
curl -k https://localhost:8080/health  # Skip verification for test

# Wrong certificate format
openssl x509 -in cert.pem -text -noout  # Verify format

# Client certificate required but not provided
curl --cert client.pem --key client-key.pem https://localhost:8080/health
```

### Performance Baselines

**Expected Performance (development)**
- PUT p95 latency: < 15ms  
- Watch emit lag p95: < 200ms
- No steady `watch_drops_total` increases
- Memory usage: < 512MB for moderate workloads

**Monitoring Queries (Prometheus):**
```promql
# P95 operation latency
histogram_quantile(0.95, rate(op_duration_seconds_bucket[5m]))

# Watch buffer health  
watch_backlog_events > 1000

# Error rates
sum(rate(agentstate_ops_total{result="error"}[5m])) / sum(rate(agentstate_ops_total[5m]))
```

---

## Quick Reference

**Health Check:** `GET /health`
**Metrics:** `GET /metrics` 
**Admin API:** `POST /admin/{snapshot,trim-wal}` (requires admin cap)

**Default Ports:**
- 8080: HTTP API
- 9090: Metrics/Admin  

**Container Images:**
- `ghcr.io/REPLACE_ORG/agentstate-server:v0.1.0-rc.1`
- `ghcr.io/REPLACE_ORG/agentstate-cli:v0.1.0-rc.1`

**Helm Chart:** `deploy/helm/` or OCI `oci://registry.com/charts/agentstate`