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

- `8080`: WebSocket stream endpoint
- `8081`: HTTP API and health endpoint

## Health check

```bash
kubectl port-forward svc/ailoop-sidecar-example 8081:8081
curl http://127.0.0.1:8081/api/v1/health
```

## Typical sidecar container block

```yaml
- name: ailoop-sidecar
  image: ailoop-cli:latest
  ports:
  - containerPort: 8080
  - containerPort: 8081
  readinessProbe:
    httpGet:
      path: /api/v1/health
      port: 8081
  livenessProbe:
    httpGet:
      path: /api/v1/health
      port: 8081
```

## Operational checks

- `kubectl describe pod <pod>`
- `kubectl get events --sort-by=.lastTimestamp`
- `kubectl get endpoints`

## Contributing

Use root workflow in `../CONTRIBUTING.md`.
