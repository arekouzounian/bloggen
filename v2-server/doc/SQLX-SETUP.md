# SQLx Compile-Time Checking Setup

This document explains how to enable SQLx's compile-time query verification.

## Why Compile-Time Checking?

SQLx can verify your SQL queries against the actual database schema at compile time, catching errors like:
- Typos in column names
- Type mismatches
- Missing tables/columns
- Invalid SQL syntax

## Current State

The project currently uses **unchecked queries** (`sqlx::query()` instead of `sqlx::query!()`) to avoid requiring a database during compilation. This works but loses compile-time safety.

---

## Option 1: Offline Mode (Recommended for CI/CD)

SQLx can cache query metadata in a `.sqlx/` directory. This allows compile-time checking without a running database.

### Setup (One-time)

```bash
# 1. Start a temporary database
docker-compose -f docker-compose.dev.yml up -d

# 2. Set DATABASE_URL
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"

# 3. Run migrations
cargo sqlx migrate run

# 4. Generate query cache
cargo sqlx prepare

# 5. Commit the .sqlx directory
git add .sqlx
git commit -m "Add sqlx query cache"

# 6. Stop the database
docker-compose -f docker-compose.dev.yml down
```

### Using in CI

```yaml
# .github/workflows/ci.yml
- name: Check sqlx queries
  run: cargo sqlx prepare --check
  env:
    SQLX_OFFLINE: true
```

### Updating Queries

After changing queries in the code:

```bash
# Start database, run migrations, regenerate cache
docker-compose -f docker-compose.dev.yml up -d
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"
cargo sqlx migrate run
cargo sqlx prepare
git add .sqlx
git commit -m "Update sqlx query cache"
```

---

## Option 2: Development Database (Recommended for Active Development)

Keep a long-running development database for continuous compile-time checking.

### Setup

```bash
# 1. Start database (runs in background)
docker-compose -f docker-compose.dev.yml up -d

# 2. Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"

# 3. Run migrations
cargo sqlx migrate run

# 4. Now cargo build will check queries against the live database
cargo build
```

### Advantages
- Real-time query validation during development
- No need to regenerate .sqlx cache for every change
- Easier to experiment with queries

### Disadvantages
- Requires PostgreSQL running locally
- Different environment than CI (unless CI also uses live DB)

---

## Option 3: Disposable Test Database (Script)

Use the provided script for a fully automated setup:

```bash
./setup-sqlx.sh
```

This script:
1. Spins up a temporary PostgreSQL container
2. Runs migrations
3. Generates `.sqlx` cache
4. Tears down the container

**Best for:** One-time setup before committing, or in CI pipelines.

---

## Option 4: Keep Unchecked Queries (Current Approach)

Continue using `sqlx::query()` instead of `sqlx::query!()`.

### Advantages
- No database required at compile time
- Faster builds
- Simpler setup

### Disadvantages
- No compile-time query validation
- Runtime errors for typos/schema mismatches
- Less IDE autocomplete support

---

## Recommendation

**For this project:**

1. **Active development:** Use Option 2 (dev database)
   - Easy to iterate on queries
   - Immediate feedback

2. **CI/CD:** Use Option 1 (offline mode)
   - Commit `.sqlx/` directory
   - Fast CI builds
   - Set `SQLX_OFFLINE=true` in CI

3. **Contributors without Docker:** Option 4 (unchecked) still works
   - Tests will catch major issues
   - Runtime errors will surface problems

---

## Converting to Checked Queries

To switch from unchecked to checked queries:

### Before (unchecked)
```rust
sqlx::query("SELECT * FROM posts WHERE slug = $1")
    .bind(slug)
    .fetch_one(&pool)
    .await?
```

### After (checked)
```rust
sqlx::query!("SELECT * FROM posts WHERE slug = $1", slug)
    .fetch_one(&pool)
    .await?
```

### Benefits of checked queries
- Column names are validated
- Return types are inferred automatically
- IDE autocomplete for result fields
- Compile-time errors for schema mismatches

---

## Environment Variables

### Development
```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"
```

### Production
```bash
export DATABASE_URL="postgres://user:password@db-host:5432/bloggen"
```

### Offline Mode (CI)
```bash
export SQLX_OFFLINE=true
```

---

## Troubleshooting

### "DATABASE_URL not set"
Set the environment variable or use offline mode.

### "relation does not exist"
Run migrations: `cargo sqlx migrate run`

### "query cache is outdated"
Regenerate: `cargo sqlx prepare`

### Port 5432 already in use
Another PostgreSQL instance is running. Either:
- Stop it: `sudo systemctl stop postgresql`
- Change port in docker-compose.dev.yml

---

## Files

- `docker-compose.dev.yml` - Development database setup
- `setup-sqlx.sh` - One-time disposable database script
- `.sqlx/` - Query cache directory (gitignore or commit based on strategy)
- `migrations/` - Database schema migrations
