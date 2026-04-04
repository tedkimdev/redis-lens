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
