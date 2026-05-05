# ailoop Kubernetes manifests

Example manifests for running ailoop as a sidecar alongside another container. For the CLI and server behavior, see the [repository README](../README.md).

## Quick deploy

```bash
kubectl apply -f deployment.yaml
kubectl get pods
kubectl get services
```

View logs:

```bash
kubectl logs -l app=ailoop-sidecar-example -c ailoop-sidecar
```

Run the integration test job:

```bash
kubectl apply -f test-job.yaml
kubectl logs -l job-name=ailoop-integration-test
```

## Current manifest status

- `deployment.yaml` exposes the unified API/WebSocket endpoint on **8080** (`Service: ailoop-sidecar-example`).
- `test-job.yaml` currently targets `http://ailoop-sidecar:8081/...` (legacy naming/port).
- `configmap.yaml` also contains legacy split-port values (`websocket_port: 8080`, `http_port: 8081`).

Before using `test-job.yaml` or `configmap.yaml` as-is, align hostnames and ports with your deployed Service.

## Exposed ports (runtime)

- **8080**: HTTP API and WebSocket (unified server runtime)

## Health check

```bash
kubectl port-forward svc/ailoop-sidecar-example 8080:8080
curl http://127.0.0.1:8080/api/v1/health
```

## Typical sidecar container block

```yaml
- name: ailoop-sidecar
  image: ailoop-cli:latest
  ports:
  - containerPort: 8080
    name: unified
  readinessProbe:
    httpGet:
      path: /api/v1/health
      port: 8080
  livenessProbe:
    httpGet:
      path: /api/v1/health
      port: 8080
```

## Operational checks

- `kubectl describe pod <pod>`
- `kubectl get events --sort-by=.lastTimestamp`
- `kubectl get endpoints`

## Contributing

Clone, build, tests, and docs live in [CONTRIBUTING.md](../CONTRIBUTING.md). Design notes: [ARCHITECTURE.md](../ARCHITECTURE.md).
