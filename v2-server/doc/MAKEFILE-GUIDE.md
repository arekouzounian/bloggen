# BlogGen v2 Server - Makefile Guide

Quick reference for the v2-server Makefile commands.

## Quick Start

### First Time Setup

```bash
# Option 1: Full setup with persistent dev database
make setup

# Option 2: CI-friendly setup (temporary database)
make setup-ci
```

### Daily Development

```bash
# Start dev database and server
make run-dev

# Run tests
make test

# Build project
make build
```

## Command Reference

### 📦 Setup Commands

| Command | Description | Use Case |
|---------|-------------|----------|
| `make setup` | Full setup: db-up + migrate + sqlx-prepare | First time setup, new developers |
| `make setup-ci` | Isolated sqlx prep using temp DB | CI/CD, GitHub Actions |
| `make sqlx-setup` | Run setup-sqlx.sh script directly | Regenerate .sqlx metadata only |

**Which setup should I use?**
- **Local development**: `make setup` - Uses persistent Docker Compose database
- **CI/CD**: `make setup-ci` - Uses temporary database that auto-cleans
- **Quick metadata refresh**: `make sqlx-setup` - Just regenerate .sqlx/

### 🗄️ Database Commands

| Command | Description |
|---------|-------------|
| `make db-up` | Start development PostgreSQL database |
| `make db-down` | Stop development database |
| `make db-reset` | Reset database (drop and recreate) |
| `make migrate` | Run database migrations |

**Database URL**: `postgres://postgres:postgres@localhost:5432/bloggen`

### 🔍 SQLx Commands

| Command | Description | When to Use |
|---------|-------------|-------------|
| `make sqlx-prepare` | Generate SQLx query cache | After adding/modifying SQL queries |
| `make sqlx-check` | Verify cache is up to date | Before committing code |

**What is SQLx offline mode?**
SQLx can verify SQL queries at compile time. This requires either:
1. A live database connection, OR
2. Pre-generated metadata in `.sqlx/` directory

The `.sqlx/` metadata allows building without a database connection.

### 🛠️ Development Commands

| Command | Description | Database Required? |
|---------|-------------|-------------------|
| `make build` | Build the project | Yes (or use `build-offline`) |
| `make build-offline` | Build using offline SQLx | No |
| `make test` | Run all tests | Auto-starts DB |
| `make test-unit` | Unit tests only | No |
| `make test-db` | Database integration tests | Auto-starts DB |
| `make run` | Run the server | Yes (manual start) |
| `make run-dev` | Auto-start DB + run server | Auto-starts DB |

**Pro tip**: Use `make run-dev` for the most convenient development experience!

### ✅ Quality Commands

| Command | Description |
|---------|-------------|
| `make check` | Run ALL checks (fmt, lint, sqlx, test) |
| `make fmt` | Format code with rustfmt |
| `make lint` | Run clippy linter |
| `make clean` | Clean build artifacts |

**Before committing**: Run `make check` to ensure everything passes.

## Common Workflows

### 1. Starting Fresh (New Clone)

```bash
cd v2-server
make setup                    # Sets up everything
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"
make build                    # Build project
make test                     # Run tests
```

### 2. Daily Development Workflow

```bash
# Terminal 1: Run server
make run-dev

# Terminal 2: Make changes, run tests
make test-unit                # Quick unit tests
make test                     # Full test suite
```

### 3. Before Committing Code

```bash
make check                    # Runs fmt-check, lint, sqlx-check, test
# If all passes, commit!
```

### 4. After Modifying SQL Queries

```bash
make sqlx-prepare             # Regenerate query metadata
git add .sqlx/                # Commit the updated metadata
```

### 5. CI/CD Pipeline

```bash
# .github/workflows/ci.yml example:
make setup-ci                 # Generate sqlx metadata
make build-offline            # Build without database
make lint                     # Run linter
# (Tests would need a database, so run those separately)
```

### 6. Resetting Database (Start Over)

```bash
make db-reset                 # Destroys and recreates DB
make migrate                  # Run migrations
make sqlx-prepare             # Regenerate metadata
```

## Environment Variables

The Makefile uses these defaults:

```bash
DB_URL=postgres://postgres:postgres@localhost:5432/bloggen
```

You can override them:

```bash
# Use different database
make run DB_URL=postgres://user:pass@host:port/dbname

# Or set in your shell
export DATABASE_URL="postgres://..."
make run
```

## Files Created by Setup

| File/Directory | Created By | Purpose |
|----------------|------------|---------|
| `.sqlx/` | `make sqlx-prepare` | SQLx query metadata for offline builds |
| `target/` | `cargo build` | Rust build artifacts |

## Troubleshooting

### "Database connection refused"

```bash
# Check if database is running
docker ps | grep bloggen-dev-db

# If not, start it
make db-up
```

### "SQLx error: database does not exist"

```bash
# Database exists but schema isn't set up
make migrate
```

### "SQLx query metadata is out of date"

```bash
# Regenerate the .sqlx/ directory
make sqlx-prepare
```

### "Port 5432 already in use"

```bash
# Another PostgreSQL instance is running
# Either stop it, or change the port in docker-compose.dev.yml
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rust-lang/setup-rust-toolchain@v1

      - name: Setup SQLx
        run: cd v2-server && make setup-ci

      - name: Check formatting
        run: cd v2-server && make fmt-check

      - name: Lint
        run: cd v2-server && make lint

      - name: Build (offline)
        run: cd v2-server && make build-offline

      - name: Run tests (with database)
        run: |
          cd v2-server
          make db-up
          make test
          make db-down
```

## setup-sqlx.sh Script

The `setup-sqlx.sh` script provides isolated SQLx metadata generation:

```bash
./setup-sqlx.sh
```

**What it does**:
1. Starts a temporary PostgreSQL container
2. Waits for it to be ready
3. Runs migrations
4. Generates `.sqlx/` metadata
5. Stops and removes the container

**When to use**:
- CI/CD pipelines
- When you don't want a persistent dev database
- Quick metadata regeneration without affecting your dev environment

## Summary

**Most common commands**:
- `make setup` - First time setup
- `make run-dev` - Daily development
- `make test` - Run tests
- `make check` - Before committing

**Single entry point**: The Makefile is your single source of truth for all server operations. No need to remember Docker commands, DATABASE_URL values, or sqlx commands - the Makefile handles it all!
