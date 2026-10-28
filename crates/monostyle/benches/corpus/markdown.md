# Deploying the service

This guide walks through a first deployment. The fenced examples below are scored as code in their own language, and the untagged fence is reported for having no language.

## Prerequisites

You need a container runtime, a registry login, and the configuration file described in the next section. The steps assume a Unix shell; on Windows, use the WSL instructions in the appendix.

## Configuration

Write the configuration before building, because the build bakes the values in:

```toml
[service]
name = "ledger"
replicas = 3

[service.limits]
cpu = "500m"
memory = "256Mi"
```

The replica count is the one value most people get wrong. Three is the minimum for a rolling deployment to keep serving while one instance restarts, and more than five rarely helps a service this size.

## Build

```sh
docker build -t registry.example.com/ledger:latest .
docker push registry.example.com/ledger:latest
```

If the push is rejected, log in again:

```
docker login registry.example.com
```

## Verify

The health endpoint returns the running version, so a quick check is enough to confirm the roll-out finished:

```typescript
const response = await fetch("https://ledger.example.com/health");
const body = await response.json();

if (body.version !== expected) {
	throw new Error(`expected ${expected}, got ${body.version}`);
}
```

### Reading the logs

Logs are structured JSON. Filter by the request identifier to follow one request through every hop:

```python
import json
import sys

for line in sys.stdin:
    event = json.loads(line)
    if event.get("request_id") == target:
        print(event["message"])
```

## Rolling back

Roll back by re-deploying the previous tag. The deployment keeps the last five images, so there is always something to return to. See [the release history](./releases.md) for the tags.

- Find the previous tag in the release history.
- Re-run the deploy step with that tag.
- Watch the health endpoint until every replica reports the old version.
