# redis-lens

Redis diagnostics CLI for engineers who don't have Grafana yet.

No agents, no dashboards, no setup — just run it against your Redis instance.

## Usage

```bash
# Analyze cache stampede risk
redis-lens scan

# Analyze memory usage by key pattern
redis-lens memory

# With options
redis-lens --url redis://:password@host:6379/ scan
redis-lens --url redis://127.0.0.1/ scan --bucket 30 --sample 0.5
redis-lens --url redis://127.0.0.1/ scan --pattern "user:*"
redis-lens --url redis://127.0.0.1/ scan --output json
```

## Commands

### scan

Analyzes key expiry distribution and detects cache stampede risk windows.

```
Options:
  --bucket <SEC>     Bucket size in seconds (default: 60)
  --sample <RATE>    Sample rate 0.0~1.0 (default: 1.0)
  --pattern <GLOB>   Only scan keys matching this pattern (e.g. user:*)
  --output <FORMAT>  Output format: text or json (default: text)
```

### memory

Analyzes memory usage grouped by key namespace prefix.

```
Options:
  --sample <RATE>    Sample rate 0.0~1.0 (default: 1.0)
  --output <FORMAT>  Output format: text or json (default: text)
```

## Environment Variables

```bash
export REDIS_URL=redis://:password@host:6379/
redis-lens scan
```

## CI/CD Integration

```bash
SCORE=$(redis-lens --url $REDIS_URL scan --output json | jq '.risk_score')
if [ "$SCORE" -gt 70 ]; then
  echo "⚠ High stampede risk detected ($SCORE/100), consider adding TTL jitter"
  exit 1
fi
```

## Production Tips

- Use `--sample 0.1` on large Redis instances to minimize load
- Run during low-traffic hours for full scans
- Use `--pattern` to focus on specific namespaces
- `SCAN` is non-blocking — safe to interrupt anytime

## When to use redis-lens vs Grafana

| | redis-lens | Grafana + Redis Exporter |
|---|---|---|
| Setup | none | complex |
| Real-time monitoring | no | yes |
| One-off diagnostics | yes | overkill |
| CI/CD integration | yes (--output json) | possible |
| Stampede risk analysis | yes | no |
| Memory pattern analysis | yes | partial |

## Roadmap

- [x] Cache stampede risk detection
- [x] Memory usage by key pattern
- [x] JSON output for CI/CD
- [x] REDIS_URL env var
- [x] --pattern flag
- [ ] --watch mode
- [ ] --throttle option
- [ ] Hot key detection
