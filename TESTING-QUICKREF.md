# BlogGen Test Quick Reference

## Quick Commands

```bash
# Run everything
./run-tests.sh              # or: make test

# Unit tests only (fast)
./run-tests.sh --unit       # or: make test-unit

# Integration tests only
./run-tests.sh --integration # or: make test-integration
```

## Common Scenarios

### During Development
```bash
make test-unit              # Quick feedback on changes
make test-client            # Client-specific tests
make test-server            # Server-specific tests
```

### Before Committing
```bash
./run-tests.sh --unit --e2e-client  # Most relevant tests
```

### Before Opening PR
```bash
make test                   # All tests
```

### Debugging Failures
```bash
./run-tests.sh --verbose    # See detailed output
./run-tests.sh --sequential # Run tests one at a time
```

## Test Types

| Test Suite      | Command                        | Time    | Needs DB? |
|-----------------|--------------------------------|---------|-----------|
| Unit (parallel) | `make test-unit`              | ~5-10s  | No        |
| Client e2e      | `make test-e2e-client`        | ~2-5s   | No        |
| Full e2e        | `make test-e2e-full`          | ~15-30s | Yes       |
| FUSE e2e        | `make test-e2e-fuse`          | ~20-40s | Yes       |

## Environment Variables

```bash
# Use existing database
SKIP_DB_SETUP=1 make test-e2e-full

# Keep database running
KEEP_DB_RUNNING=1 make test-e2e-full

# Custom server port
SERVER_PORT=8080 make test-e2e-full
```

## See Also

- [TESTING.md](TESTING.md) - Comprehensive testing guide
- [tst/README.md](tst/README.md) - Integration test details
- [README.md](README.md) - Project overview
