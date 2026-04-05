# redis-lens

Redis stampede risk analyzer. No Grafana needed.

## Install

cargo install redis-lens

## Usage

redis-lens --url redis://127.0.0.1/
redis-lens --url redis://:password@host:6379/ --bucket 30 --sample 0.5

## Options

--url      Redis connection URL (default: redis://127.0.0.1/)
--bucket   Bucket size in seconds (default: 60)
--sample   Sample rate 0.0~1.0 (default: 1.0)

```
v0.1 — MVP
  - [ ] Affected keys list        ← current
  - [ ] Color output (red/yellow/green)
  - [ ] cargo install ready

v0.2 — Usability
  - [ ] REDIS_URL env var support
  - [ ] --output json (for CI/CD pipelines)
  - [ ] --pattern flag (scan only user:* keys)

v0.3 — Expand beyond stampede
  - [ ] Hot key detection
  - [ ] Memory usage per key pattern
  - [ ] --watch mode (live refresh)
```