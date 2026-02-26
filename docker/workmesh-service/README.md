# WorkMesh Service Container

Build image:

```bash
docker build -f docker/workmesh-service/Dockerfile -t workmesh-service:local .
```

Run container:

```bash
docker run --rm -it \
  -p 4747:4747 \
  -v "$PWD:/workspace" \
  -v "$HOME/.workmesh:/home/workmesh/.workmesh" \
  workmesh-service:local \
  --host 0.0.0.0 \
  --port 4747 \
  --auth-token "<token>"
```

Compose sample:

```bash
cd docker/workmesh-service
WORKMESH_REPO_ROOT=/absolute/path/to/repo \
WORKMESH_AUTH_TOKEN=<token> \
docker compose up --build -d
```

Probe health:

```bash
curl -H "Authorization: Bearer <token>" \
  http://127.0.0.1:4747/v1/healthz
```
