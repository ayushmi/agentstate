## v0.1.0-rc.1 (2025-08-21)

### Status: Pre-Release (Development Build)

This RC contains the deployment infrastructure and storage layer foundations, but requires additional development work before being production-ready.

### ✅ Completed Components

**Storage Layer:**
- `agentstate-storage` crate builds successfully  
- Async trait-based Storage interface with ?Send annotation
- In-memory MVCC implementation with borrow-checker fixes
- WAL (Write-Ahead Log) binary format and writer
- Snapshot/restore infrastructure  
- Watch streams with buffering and overflow handling

**Deployment & Documentation:**
- Complete deployment guide ([docs/DEPLOY.md](docs/DEPLOY.md))
- Docker Compose configuration with health checks
- Helm chart (v0.1.0-rc.1) with configurable values
- Updated README with quickstart examples
- Preflight validation script (`scripts/preflight.sh`)
- Grafana dashboard JSON

### ⚠️ Known Issues & Limitations

**Server/CLI Compilation Blockers:**
- `agentstate-server` has 50+ compilation errors requiring:
  - Dependency version alignment (opentelemetry, tonic TLS features)  
  - gRPC configuration updates
  - Type system integration fixes (HashMap vs BTreeMap, async trait propagation)
  - ?Send trait constraint propagation through service layers

**Architecture Considerations:**
- Storage trait uses ?Send to handle async mutex constraints (temporary fix)
- Server layer needs async trait redesign for Send compatibility
- Missing dependency features in Cargo.toml files

### 🚧 Development Path Forward

**Immediate (Required for RC.2):**
1. Fix server dependencies and gRPC configuration
2. Resolve async trait Send constraints properly  
3. Complete type system integration
4. Validate Docker build process

**Future Releases:**
- Production-ready async architecture
- Complete SDK implementations
- End-to-end integration testing
- Performance benchmarking

### 📦 Deployment Infrastructure Ready

Despite compilation issues, the following deployment artifacts are ready:

```bash
# Helm chart
helm package deploy/helm  # → agentstate-0.1.0-rc.1.tgz

# Docker Compose  
docker-compose up  # (when compilation issues resolved)

# Documentation
docs/DEPLOY.md     # Complete 10-min Docker + 20-min Helm guide
```

### Migration Notes

When upgrading to RC.2+:
- Storage interface may change as Send constraints are resolved
- Docker image names will be finalized  
- Helm values.yaml structure is stable

