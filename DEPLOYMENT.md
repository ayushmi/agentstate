# 🚀 AgentState Deployment Guide

Complete guide for deploying AgentState in production environments.

## 🎯 Deployment Options

### 🐳 Docker (Quick Start)

**Basic deployment:**
```bash
docker run -p 8080:8080 -p 9090:9090 agentstate:v1.0.0
```

**Production with persistent storage:**
```bash
docker run -d --name agentstate \
  -p 8080:8080 -p 9090:9090 \
  -e DATA_DIR=/data \
  -v agentstate-data:/data \
  --restart unless-stopped \
  agentstate:v1.0.0
```

### 🐙 Docker Compose

Create `docker-compose.yml`:
```yaml
version: '3.8'
services:
  agentstate:
    image: agentstate:v1.0.0
    ports:
      - "8080:8080"
      - "9090:9090"
    environment:
      - DATA_DIR=/data
      - LOG_LEVEL=info
    volumes:
      - agentstate-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

volumes:
  agentstate-data:
```

Deploy:
```bash
docker-compose up -d
```

### ☸️ Kubernetes (Recommended for Production)

**Quick deployment:**
```bash
kubectl apply -f deploy/kubernetes/agentstate-deployment.yaml
```

**With ingress and monitoring:**
```bash
kubectl apply -f deploy/kubernetes/
kubectl apply -f deploy/monitoring/
```

**Custom configuration:**
```bash
# Create namespace
kubectl create namespace agentstate

# Deploy with custom values
helm install agentstate ./deploy/helm/agentstate \
  --namespace agentstate \
  --set image.tag=v1.0.0 \
  --set persistence.enabled=true \
  --set persistence.size=100Gi \
  --set ingress.enabled=true \
  --set ingress.host=api.agentstate.example.com
```

## ⚙️ Configuration

### Environment Variables

| Variable | Description | Default | Production |
|----------|-------------|---------|------------|
| `DATA_DIR` | Persistent storage directory | - | `/data` |
| `LOG_LEVEL` | Logging level | `info` | `info` |
| `OTLP_ENDPOINT` | OpenTelemetry endpoint | - | `http://jaeger:14268` |

### Resource Requirements

**Minimum (Development):**
- CPU: 100m
- Memory: 256Mi
- Storage: 10Gi

**Recommended (Production):**
- CPU: 1000m (1 core)
- Memory: 1Gi
- Storage: 100Gi SSD
- Replicas: 3+

### Port Configuration

- **8080**: HTTP API (REST endpoints)
- **9090**: gRPC API (streaming/high-performance)

## 🔒 Security Configuration

### TLS/HTTPS Setup

**Using Let's Encrypt with cert-manager:**
```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: agentstate-ingress
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - api.agentstate.example.com
    secretName: agentstate-tls
  rules:
  - host: api.agentstate.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: agentstate
            port:
              number: 8080
```

### Network Security

**Network Policy (Kubernetes):**
```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: agentstate-network-policy
spec:
  podSelector:
    matchLabels:
      app: agentstate
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - {} # Allow all outbound for now
```

### Container Security

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 1000
  fsGroup: 1000
  capabilities:
    drop:
    - ALL
  readOnlyRootFilesystem: true
```

## 📊 Monitoring & Observability

### Prometheus Metrics

AgentState exposes metrics at `/metrics`:

**Key metrics to monitor:**
- `agentstate_ops_total` - Total operations
- `op_duration_seconds` - Operation latency
- `wal_active_segments` - WAL segments
- `watch_clients` - Active watch connections

**Prometheus scrape config:**
```yaml
- job_name: 'agentstate'
  kubernetes_sd_configs:
  - role: endpoints
    namespaces:
      names:
      - agentstate
  relabel_configs:
  - source_labels: [__meta_kubernetes_service_name]
    action: keep
    regex: agentstate
```

### Grafana Dashboard

Import the dashboard from `deploy/grafana/agentstate-dashboard.json` or use the ConfigMap:

```bash
kubectl apply -f deploy/monitoring/monitoring.yaml
```

### Alerting Rules

**Critical alerts:**
- Service down (5+ minutes)
- High latency (P95 > 100ms)
- High error rate (> 1%)
- Resource exhaustion (CPU/Memory > 80%)

### Logging

**Structured JSON logs:**
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "message": "Agent created",
  "agent_id": "01HKXM...",
  "namespace": "production",
  "operation": "create"
}
```

**Log aggregation with Fluentd/Loki:**
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/containers/agentstate-*.log
      pos_file /tmp/agentstate.log.pos
      tag agentstate.*
      format json
      time_key timestamp
    </source>
    
    <match agentstate.**>
      @type loki
      url http://loki:3100
      <label>
        service agentstate
        namespace #{ENV['NAMESPACE']}
      </label>
    </match>
```

## 🔄 High Availability & Scaling

### Multi-Replica Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agentstate
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  template:
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values:
                  - agentstate
              topologyKey: kubernetes.io/hostname
```

### Load Balancing

**Service configuration:**
```yaml
apiVersion: v1
kind: Service
metadata:
  name: agentstate
spec:
  type: ClusterIP
  sessionAffinity: None  # Round-robin load balancing
  ports:
  - port: 8080
    targetPort: 8080
  selector:
    app: agentstate
```

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: agentstate-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: agentstate
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

## 💾 Backup & Disaster Recovery

### Data Backup Strategy

**Persistent Volume Snapshots:**
```yaml
apiVersion: snapshot.storage.k8s.io/v1
kind: VolumeSnapshot
metadata:
  name: agentstate-snapshot
spec:
  source:
    persistentVolumeClaimName: agentstate-data
  volumeSnapshotClassName: csi-snapclass
```

**Automated backup with CronJob:**
```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: agentstate-backup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: backup-tool:latest
            command:
            - /bin/sh
            - -c
            - |
              kubectl create volumesnapshot agentstate-snapshot-$(date +%Y%m%d) \
                --from-pvc=agentstate-data
          restartPolicy: OnFailure
```

### Disaster Recovery Plan

1. **Monitor backup health** - Verify snapshots are created successfully
2. **Test restore procedures** - Regular restore tests to staging
3. **Document RTO/RPO** - Recovery time/point objectives
4. **Multi-region deployment** - For critical applications

## 🚀 Production Deployment Workflow

### 1. Pre-deployment Checks

```bash
# Run complete verification
make verify

# Test Docker image
docker run --rm -p 8080:8080 agentstate:v1.0.0 &
sleep 5
curl -f http://localhost:8080/health
pkill agentstate-server
```

### 2. Staging Deployment

```bash
# Deploy to staging namespace
kubectl apply -f deploy/kubernetes/agentstate-deployment.yaml -n staging

# Run integration tests against staging
export AGENTSTATE_URL=https://staging-api.agentstate.example.com
python integration_tests.py
```

### 3. Production Deployment

```bash
# Deploy to production
kubectl apply -f deploy/kubernetes/ -n production

# Verify deployment
kubectl get pods -n production
kubectl logs -n production deployment/agentstate

# Run smoke tests
curl -f https://api.agentstate.example.com/health
```

### 4. Post-deployment Validation

- [ ] Health checks passing
- [ ] Metrics flowing to Prometheus
- [ ] Dashboards showing healthy status
- [ ] Alerts configured and tested
- [ ] Load testing passed
- [ ] Documentation updated

## 🛠️ Troubleshooting

### Common Issues

**Container won't start:**
```bash
# Check pod status
kubectl describe pod -l app=agentstate -n agentstate

# Check logs
kubectl logs -l app=agentstate -n agentstate --previous
```

**High latency:**
```bash
# Check metrics
curl http://localhost:8080/metrics | grep -E "(latency|duration)"

# Check resource usage
kubectl top pods -n agentstate
```

**Storage issues:**
```bash
# Check PVC status
kubectl get pvc -n agentstate

# Check storage usage
kubectl exec -it deployment/agentstate -n agentstate -- df -h /data
```

### Performance Tuning

**Optimize for high write throughput:**
- Use SSD storage with high IOPS
- Increase memory limits for caching
- Tune batch sizes in applications

**Optimize for low latency:**
- Use local SSD storage
- Increase CPU limits
- Optimize client connection pooling

## 📞 Support & Maintenance

### Regular Maintenance Tasks

- **Security updates**: Monthly image updates
- **Performance monitoring**: Weekly performance reviews  
- **Backup verification**: Weekly restore tests
- **Capacity planning**: Monthly resource usage analysis
- **Documentation**: Keep deployment docs current

### Getting Help

- **GitHub Issues**: [Repository Issues](https://github.com/your-org/agentstate/issues)
- **Documentation**: Comprehensive guides in `/docs`
- **Monitoring**: Use dashboards and alerts for early detection
- **Community**: Join discussions and share experiences

---

**🎯 Ready to deploy AgentState in production!** Follow this guide for a secure, reliable, and scalable deployment.