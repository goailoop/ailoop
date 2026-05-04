# ailoop Kubernetes manifests

Kubernetes deployment examples for running ailoop as a sidecar or service.

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

Run integration test job:

```bash
kubectl apply -f test-job.yaml
kubectl logs -l job-name=ailoop-integration-test
```

## Exposed ports

- `8080`: Unified WebSocket + HTTP API endpoint

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

Use root workflow in `../CONTRIBUTING.md`.
