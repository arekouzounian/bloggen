# BlogGen Testing Guide

This document provides comprehensive information about running tests in the BlogGen project.

## Table of Contents

- [Quick Start](#quick-start)
- [Test Categories](#test-categories)
- [Running Tests](#running-tests)
- [Test Options](#test-options)
- [Environment Variables](#environment-variables)
- [Continuous Integration](#continuous-integration)
- [Troubleshooting](#troubleshooting)

## Quick Start

The simplest way to run all tests:

```bash
# Run everything (unit + integration tests)
./run-tests.sh

# Or use Make
make test
```

## Test Categories

### Unit Tests

Unit tests verify individual components in isolation. These are fast and don't require external dependencies like databases.

**Components tested:**
- Client (`client/bgc`): 106+ tests
  - Markdown parsing and CAS generation
  - Serialization/deserialization (JSON, MessagePack)
  - Compression (zstd)
  - Delta computation
  - Rendering

- Server (`server`): 33+ tests
  - Configuration management
  - Error handling
  - HTML/Markdown rendering
  - Storage walker
  - Garbage collection

### Integration Tests

Integration tests validate complete workflows end-to-end. These tests use real databases and network connections.

**Test suites:**

1. **e2e-client** (Client-only, ~2-5 seconds)
   - No server required
   - Tests client parsing, serialization, compression
   - Round-trip fidelity tests
   - Delta computation and edge cases

2. **e2e-full** (Full stack, ~15-30 seconds)
   - Requires PostgreSQL and server
   - Tests complete upload/download workflow
   - Tests delta updates
   - Verifies server persistence

3. **e2e-fuse** (FUSE filesystem, ~20-40 seconds)
   - Requires PostgreSQL, server, and FUSE support
   - Tests virtual filesystem mounting
   - Tests file create/read/write through FUSE
   - Tests persistence across mount/unmount cycles

## Running Tests

### Using the Unified Test Runner

The `run-tests.sh` script provides a unified interface:

```bash
# Run all tests
./run-tests.sh

# Run only unit tests
./run-tests.sh --unit

# Run only integration tests
./run-tests.sh --integration

# Run specific integration test
./run-tests.sh --e2e-client
./run-tests.sh --e2e-full
./run-tests.sh --e2e-fuse

# Run with verbose output
./run-tests.sh --verbose

# Run unit tests sequentially instead of parallel
./run-tests.sh --sequential
```

### Using Make

Convenient shortcuts via Makefile:

```bash
# Testing
make test              # Run all tests
make test-unit         # Run unit tests only
make test-integration  # Run integration tests only
make test-client       # Run client unit tests only
make test-server       # Run server unit tests only
make test-e2e-client   # Run client e2e tests
make test-e2e-full     # Run full integration tests
make test-e2e-fuse     # Run FUSE integration tests

# Advanced
make test-verbose      # Run all tests with verbose output
make test-sequential   # Run unit tests sequentially

# Building
make build             # Build all components
make build-client      # Build client only
make build-server      # Build server only

# Cleaning
make clean             # Clean all build artifacts
```

### Using Cargo Directly

For fine-grained control:

```bash
# Build entire workspace
cargo build --workspace

# Test entire workspace
cargo test --workspace

# Test specific package
cargo test -p bgc
cargo test -p server

# Run specific test
cargo test -p bgc test_parse_markdown_file

# Run tests with output
cargo test -- --nocapture

# Run tests with threads=1 (sequential)
cargo test -- --test-threads=1
```

## Test Options

### Parallel vs Sequential Execution

By default, unit tests run in parallel for faster execution:

```bash
# Parallel (default, faster)
./run-tests.sh --unit

# Sequential (better for debugging)
./run-tests.sh --unit --sequential
```

### Verbose Output

For debugging test failures:

```bash
./run-tests.sh --verbose
```

This shows:
- Full cargo test output
- Detailed logs from integration tests
- Real-time progress

### Combining Options

Options can be combined:

```bash
# Run unit + specific e2e test with verbose output
./run-tests.sh --unit --e2e-client --verbose

# Run all integration tests sequentially with verbose output
./run-tests.sh --integration --sequential --verbose
```

## Environment Variables

Integration tests support configuration via environment variables:

### Database Setup

```bash
# Skip database setup (assumes DB already running)
SKIP_DB_SETUP=1 ./run-tests.sh --e2e-full

# Keep database running after tests (for inspection)
KEEP_DB_RUNNING=1 ./run-tests.sh --e2e-full
```

### Server Configuration

```bash
# Use custom server port
SERVER_PORT=8080 ./run-tests.sh --e2e-full
```

### Combined Example

```bash
# Run integration tests with existing DB on custom port
SKIP_DB_SETUP=1 SERVER_PORT=8080 ./run-tests.sh --integration
```

## Continuous Integration

### GitHub Actions

The repository includes a CI workflow (`.github/workflows/test.yml`) that:
- Runs on push to `main` and `v2` branches
- Runs on pull requests
- Caches Rust dependencies
- Runs unit tests in parallel
- Runs integration tests separately

### Running CI Locally

Simulate CI environment:

```bash
# Clean build
make clean
make build

# Run tests as CI would
./run-tests.sh --unit
./run-tests.sh --e2e-client
./run-tests.sh --e2e-full
```

## Troubleshooting

### Common Issues

#### 1. Build Failures

```bash
# Clean and rebuild
cargo clean
cargo build --workspace

# Check for dependency issues
cargo update
cargo build --workspace
```

#### 2. Binary Not Found

If e2e tests fail with "binary not found":

```bash
# Ensure workspace is built
cargo build --workspace

# Check binary location
ls -la target/debug/bgc
ls -la target/debug/server
```

#### 3. Database Connection Errors

```bash
# Check if PostgreSQL is running
docker ps | grep bloggen-dev-db

# Start database manually
cd server
docker compose -f docker-compose.dev.yml up -d

# Run tests with existing DB
SKIP_DB_SETUP=1 ./run-tests.sh --e2e-full
```

#### 4. Port Already in Use

```bash
# Use different port
SERVER_PORT=8080 ./run-tests.sh --e2e-full

# Or kill process using port 3000
lsof -ti:3000 | xargs kill -9
```

#### 5. FUSE Tests Failing

FUSE tests require:
- `libfuse3-dev` installed
- `fuse3` kernel module loaded
- User permissions for FUSE

```bash
# Install FUSE on Ubuntu/Debian
sudo apt-get install libfuse3-dev fuse3

# Check FUSE module
lsmod | grep fuse
```

### Getting Help

For detailed test output:

```bash
# Run with verbose flag
./run-tests.sh --verbose

# Check test logs (for integration tests)
# Logs are in /tmp/tmp.XXXXXX/server.log during test execution
```

For individual test debugging:

```bash
# Run specific test with output
cargo test -p bgc test_name -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test -p bgc test_name
```

## Performance

Typical execution times:

| Test Suite      | Time         | Requirements           |
|-----------------|--------------|------------------------|
| Unit tests      | ~5-10s       | None                   |
| e2e-client      | ~2-5s        | None                   |
| e2e-full        | ~15-30s      | Docker, PostgreSQL     |
| e2e-fuse        | ~20-40s      | Docker, PostgreSQL, FUSE |
| **All tests**   | ~40-80s      | Docker, PostgreSQL, FUSE |

Parallel unit test execution saves ~5-10 seconds compared to sequential.

## Test Structure

```
bloggen/
├── client/
│   ├── bgc/
│   │   └── src/
│   │       ├── lib.rs       # Unit tests inline
│   │       ├── cas.rs       # Unit tests inline
│   │       └── ...
│   └── bgc-ast/
│       └── src/
│           └── lib.rs
├── server/
│   └── src/
│       ├── main.rs
│       ├── config.rs        # Unit tests inline
│       └── ...
├── tst/
│   ├── README.md            # Integration test docs
│   ├── e2e-client.sh        # Client-only tests
│   ├── e2e-full.sh          # Full stack tests
│   └── e2e-fuse.sh          # FUSE tests
├── run-tests.sh             # Unified test runner
├── Makefile                 # Convenient shortcuts
├── Cargo.toml               # Workspace definition
└── .github/
    └── workflows/
        └── test.yml         # CI configuration
```

## Best Practices

### For Development

1. **Run unit tests frequently** during development:
   ```bash
   make test-unit
   ```

2. **Run relevant integration test** before committing:
   ```bash
   ./run-tests.sh --e2e-client  # For client changes
   ./run-tests.sh --e2e-full    # For server changes
   ```

3. **Run all tests** before opening a PR:
   ```bash
   make test
   ```

### For CI/CD

1. **Run unit tests first** (fast feedback):
   ```bash
   ./run-tests.sh --unit
   ```

2. **Run integration tests in parallel** when possible:
   ```bash
   ./run-tests.sh --e2e-client &
   ./run-tests.sh --e2e-full &
   wait
   ```

3. **Keep database running** between test runs:
   ```bash
   KEEP_DB_RUNNING=1 ./run-tests.sh --e2e-full
   SKIP_DB_SETUP=1 ./run-tests.sh --e2e-full  # Subsequent runs
   ```

### For Debugging

1. **Use verbose mode**:
   ```bash
   ./run-tests.sh --verbose
   ```

2. **Run tests sequentially**:
   ```bash
   ./run-tests.sh --unit --sequential
   ```

3. **Run specific test with cargo**:
   ```bash
   cargo test -p bgc test_name -- --nocapture
   ```

## Contributing

When adding new tests:

1. **Unit tests**: Add to existing test modules or create new ones
2. **Integration tests**: Update existing e2e scripts or add new ones
3. **Update documentation**: Keep README.md and this guide up to date
4. **Verify CI**: Ensure tests pass in CI environment

---

For more details on integration tests, see [tst/README.md](../tst/README.md).
