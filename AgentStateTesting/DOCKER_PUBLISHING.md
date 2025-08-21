# Docker Image Publishing Guide

## Current Status

The AgentState Docker image is built and ready for publishing. The testing environment currently uses a locally built image.

## Built Images

- **Local**: `agentstate/server:latest` and `agentstate/server:1.0.0`
- **Size**: ~52MB (optimized distroless image)

## Publishing to Docker Hub

### Prerequisites

1. Docker Hub account with appropriate permissions
2. Docker CLI logged in: `docker login`

### Publishing Commands

```bash
# Tag for Docker Hub (replace with actual organization/username)
docker tag agentstate/server:latest ayushsmittal/agentstate:latest
docker tag agentstate/server:1.0.0 ayushsmittal/agentstate:1.0.0

# Push to Docker Hub
docker push ayushsmittal/agentstate:latest
docker push ayushsmittal/agentstate:1.0.0
```

### Update docker-compose.yml

Once published, update the testing docker-compose.yml:

```yaml
services:
  agentstate:
    image: ayushsmittal/agentstate:latest
    # Remove the build section
```

## Alternative Registries

### GitHub Container Registry

```bash
# Tag for GitHub
docker tag agentstate/server:latest ghcr.io/ayushmi/agentstate:latest

# Push to GitHub (requires authentication)
docker push ghcr.io/ayushmi/agentstate:latest
```

### Private Registry

```bash
# Tag for private registry
docker tag agentstate/server:latest your-registry.com/agentstate:latest

# Push to private registry
docker push your-registry.com/agentstate:latest
```

## For Users

Once published, users can run AgentState with:

```bash
# Using docker run
docker run -p 8080:8080 -e DATA_DIR=/data -v agentstate-data:/data ayushsmittal/agentstate:latest

# Using docker-compose (in AgentStateTesting)
docker-compose up -d agentstate
```

## Testing Published Image

```bash
# Pull and test the published image
docker pull ayushsmittal/agentstate:latest
docker run -p 8080:8080 ayushsmittal/agentstate:latest

# Test health endpoint
curl http://localhost:8080/health
```

## Notes

- The image is built from the main AgentState codebase using `docker/Dockerfile`
- Uses distroless base image for security and minimal size
- Includes all necessary dependencies for the Rust server
- Exposes port 8080 by default
- Supports persistent storage via volume mounts