# Ailoop Kubernetes Deployment

This directory contains Kubernetes manifests for deploying ailoop as a sidecar container in Kubernetes environments.

## Quick Start

```bash
# Deploy the example sidecar setup
kubectl apply -f deployment.yaml

# Check deployment status
kubectl get pods
kubectl get services

# View logs
kubectl logs -l app=ailoop-sidecar-example -c ailoop-sidecar

# Test the deployment
kubectl apply -f test-job.yaml
kubectl logs -l job-name=ailoop-integration-test
```

## Architecture

The ailoop sidecar pattern provides:

- **WebSocket server** (port 8080): Real-time bidirectional communication
- **HTTP API server** (port 8081): REST endpoints for message operations
- **Health checks**: Kubernetes-ready health endpoints
- **Security**: Non-root execution, minimal privileges

## Deployment Options

### 1. Basic Sidecar Deployment

Use `deployment.yaml` for a complete sidecar setup with:
- Main application container (nginx example)
- Ailoop sidecar container
- Service for external access
- Network policy for security

### 2. Sidecar-Only Deployment

For existing applications, deploy only the sidecar:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-with-ailoop
spec:
  template:
    spec:
      containers:
      # Your existing app
      - name: my-app
        image: my-app:latest
        # ... your app config

      # Add ailoop sidecar
      - name: ailoop-sidecar
        image: ailoop-cli:latest
        ports:
        - containerPort: 8080
          name: websocket
        - containerPort: 8081
          name: http-api
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /api/v1/health
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 10
```

## Configuration

### Environment Variables

```yaml
env:
- name: RUST_LOG
  value: "info"  # debug, info, warn, error
- name: AILOOP_HOST
  value: "0.0.0.0"
- name: AILOOP_PORT
  value: "8080"
```

### ConfigMap

Use `configmap.yaml` for advanced configuration:

```bash
kubectl apply -f configmap.yaml
```

Then mount it in your deployment:

```yaml
volumeMounts:
- name: config
  mountPath: /etc/ailoop
  readOnly: true
volumes:
- name: config
  configMap:
    name: ailoop-config
```

## Health Checks

The sidecar provides comprehensive health checks:

```bash
# HTTP health endpoint
curl http://localhost:8081/api/v1/health

# Response:
{
  "status": "healthy",
  "version": "0.1.1",
  "activeConnections": 0,
  "queueSize": 0,
  "activeChannels": 0
}
```

## Scaling Considerations

- **Horizontal scaling**: Each pod gets its own sidecar instance
- **State management**: Messages are stored per-pod (consider persistent storage for production)
- **Load balancing**: Use Kubernetes services for distributing WebSocket connections

## Security

The deployment includes security best practices:

- **Non-root user**: Runs as user 65532 (distroless nonroot)
- **Read-only filesystem**: No write access to container filesystem
- **Minimal capabilities**: All capabilities dropped
- **Network policies**: Restrict ingress/egress traffic
- **Resource limits**: Memory and CPU constraints

## Monitoring

### Metrics

Monitor these key metrics:

```bash
# Active connections
curl http://localhost:8081/api/v1/health | jq .activeConnections

# Queue size
curl http://localhost:8081/api/v1/health | jq .queueSize

# Active channels
curl http://localhost:8081/api/v1/health | jq .activeChannels
```

### Logs

```bash
# View sidecar logs
kubectl logs -l app=ailoop-sidecar-example -c ailoop-sidecar

# Follow logs
kubectl logs -f -l app=ailoop-sidecar-example -c ailoop-sidecar
```

## Troubleshooting

### Pod won't start
```bash
# Check events
kubectl describe pod <pod-name>

# Check logs
kubectl logs <pod-name> -c ailoop-sidecar
```

### Health check failures
```bash
# Exec into pod to debug
kubectl exec -it <pod-name> -c ailoop-sidecar -- sh

# Test health endpoint
curl http://localhost:8081/api/v1/health
```

### Connection issues
```bash
# Check service endpoints
kubectl get endpoints

# Test WebSocket connection from another pod
kubectl exec -it <test-pod> -- websocat ws://ailoop-sidecar:8080
```

## Production Considerations

### High Availability
```yaml
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
```

### Resource Optimization
```yaml
resources:
  requests:
    memory: "32Mi"
    cpu: "10m"
  limits:
    memory: "64Mi"
    cpu: "100m"
```

### Persistent Storage
For production, consider persistent message storage:

```yaml
volumes:
- name: message-storage
  persistentVolumeClaim:
    claimName: ailoop-messages
```

## SDK Integration Examples

### TypeScript SDK
```typescript
import { AiloopClient } from 'ailoop-js';

const client = new AiloopClient({
  baseURL: 'http://localhost:8081'
});

// In Kubernetes, use the service name
const k8sClient = new AiloopClient({
  baseURL: 'http://ailoop-sidecar:8081'
});
```

### Python SDK
```python
from ailoop import AiloopClient

# In Kubernetes
client = AiloopClient(base_url='http://ailoop-sidecar:8081')
```

This setup enables seamless communication between your application containers and the ailoop sidecar for real-time AI agent interactions!
