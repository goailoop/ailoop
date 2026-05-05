# Ailoop Containerization

This document describes how to build and run ailoop using Docker for sidecar deployments.

## Building the Docker Image

```bash
# Build the ailoop-cli Docker image
docker build -t ailoop-cli:latest .

# Or build with a specific tag
docker build -t ailoop-cli:v0.1.1 .
```

## Running the Container

```bash
# Run with default settings (single unified port)
docker run -p 8080:8080 ailoop-cli:latest

# Run with custom environment variables
docker run \
  -p 8080:8080 \
  -e RUST_LOG=debug \
  ailoop-cli:latest
```

The container exposes:
- **Port 8080**: Unified server — WebSocket (`ws://`) and HTTP API (`http://`) on the same port

## Testing the Sidecar Pattern

Use the provided docker-compose setup to test the sidecar deployment pattern:

```bash
# Start the complete sidecar setup
docker-compose up

# Or run in background
docker-compose up -d
```

This will start:
1. **ailoop-sidecar**: The ailoop server container
2. **app**: A sample web application container
3. **test-client**: Optional Node.js test client

## Health Checks

The container includes health check endpoints:

```bash
# Check server health
curl http://localhost:8080/api/v1/health

# Expected response:
{
  "status": "healthy",
  "version": "0.1.1",
  "activeConnections": 0,
  "queueSize": 0,
  "activeChannels": 0
}
```

## Kubernetes Deployment

For production Kubernetes deployments, use the following pattern:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-with-ailoop
spec:
  replicas: 1
  selector:
    matchLabels:
      app: my-app
  template:
    metadata:
      labels:
        app: my-app
    spec:
      containers:
      - name: my-app
        image: my-app:latest
        ports:
        - containerPort: 3000
        env:
        - name: AILOOP_BASE_URL
          value: "http://localhost:8080"
      - name: ailoop-sidecar
        image: ailoop-cli:latest
        ports:
        - containerPort: 8080
          name: unified
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Environment Variables

The ailoop container supports these environment variables:

- `RUST_LOG`: Set logging level (error, warn, info, debug, trace)
- `AILOOP_HOST`: Bind host (default: 0.0.0.0)
- `AILOOP_PORT`: Server port (default: 8080)

## Image Optimization

The Docker image is optimized for minimal size:

- **Multi-stage build**: Separate build and runtime stages
- **Alpine builder**: Small build environment
- **Distroless runtime**: Minimal runtime with no shell or package manager
- **Binary stripping**: Debug symbols removed from binary
- **No unnecessary files**: Only the ailoop binary is included

Result: Image size under 30MB (target achieved!)

## Development

For development with live reloading:

```bash
# Mount source code and rebuild on changes
docker run \
  -v $(pwd):/app \
  -w /app \
  rust:1.75-slim \
  cargo build --release --bin ailoop
```

## Troubleshooting

### Container won't start
```bash
# Check container logs
docker logs <container-id>

# Check if port is available
netstat -tlnp | grep :8080
```

### Health check fails
```bash
# Test health endpoint manually
curl -v http://localhost:8080/api/v1/health

# Check if binary is working
docker run --rm ailoop-cli:latest ailoop --help
```

### WebSocket connection issues
```bash
# Test WebSocket connection
websocat ws://localhost:8080
```

## Security Considerations

- Container runs as non-root user
- Minimal attack surface (distroless image)
- No shell or package manager in runtime image
- Binary is stripped of debug symbols
