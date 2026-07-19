# Go job-service baseline

This directory contains the M4 Go baseline for the frozen UC-001 and UC-003
job-service benchmark.

- `v1/` is the buildable and tested UC-001 checkpoint used as the starting
  source for the priority-evolution task.
- `v2/` is the verified UC-003 reference implementation and benchmark runner.
- `checkpoints.json` freezes both source trees with reproducible SHA-256
  digests.
- `seed-locations.json` identifies stable Go semantic locations where M7 can
  instantiate the frozen hidden seed categories.
- `runner.json` and the locked verification manifest connect V2 to the
  language-neutral M2 harness.

The module uses stable Go 1.26.0 and only the standard library. Building,
testing, and running fixtures do not require dependency downloads or network
access.

## Setup

Install Go 1.26.0 and `gopls` 0.21.1. Confirm the selected tools:

```bash
go version
gopls version
```

If the normal Go build cache is not writable, set `GOCACHE` to a writable
temporary directory:

```bash
export GOCACHE=/tmp/ail-go-build-cache
```

## Build and checks

Run these commands from the repository root:

```bash
go -C benchmarks/baselines/go build ./...
gofmt -d benchmarks/baselines/go
go -C benchmarks/baselines/go vet ./...
go -C benchmarks/baselines/go test ./...
go -C benchmarks/baselines/go test -race ./...
gopls check \
  benchmarks/baselines/go/v1/jobservice.go \
  benchmarks/baselines/go/v2/domain/domain.go \
  benchmarks/baselines/go/v2/service/service.go \
  benchmarks/baselines/go/v2/store/store.go \
  benchmarks/baselines/go/v2/fixture/codec.go \
  benchmarks/baselines/go/v2/cmd/runner/main.go
python3 benchmarks/tools/harness.py verify \
  --language go \
  --visibility public
```

The tests replay all 37 shared public fixtures and add focused checks for:

- field bounds, validation ordering, and effect-free failures;
- the closed public result and store-outcome sets;
- exact one-call persistence behavior and defensive state copies;
- all three priority identities and unchanged propagation;
- V1 request and persisted-record adaptation to normal;
- V1 response projection and V2 persisted encoding;
- unknown-priority boundary failures;
- runner argument, JSON, digest, ordering, and process contracts;
- checkpoint source-tree digests; and
- every frozen hidden seed category.

## Coverage

Generate and inspect statement coverage with:

```bash
go -C benchmarks/baselines/go test \
  -coverprofile=/tmp/ail-go-coverage.out \
  ./...
go -C benchmarks/baselines/go tool cover \
  -func=/tmp/ail-go-coverage.out
```

## Run one case

```bash
go -C benchmarks/baselines/go run ./v2/cmd/runner \
  --case benchmarks/fixtures/public/create_job/uc003-v2-created-priority-high.json
```

## Run the corpus

```bash
go -C benchmarks/baselines/go run ./v2/cmd/runner \
  --corpus benchmarks/fixtures/manifest.json
```

Each runner command writes exactly one normalized JSON value to standard
output. Diagnostics are written to standard error.
