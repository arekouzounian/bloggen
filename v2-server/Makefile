# BlogGen v2 Server - Development Makefile

.PHONY: help db-up db-down db-reset sqlx-prepare sqlx-check sqlx-setup migrate \
        test test-unit test-integration test-db build build-offline run run-dev \
        clean setup setup-ci check fmt lint

# Default target
help:
	@echo "BlogGen v2 Server - Available commands:"
	@echo ""
	@echo "📦 Setup:"
	@echo "  make setup          - Full setup: db-up + migrate + sqlx-prepare"
	@echo "  make setup-ci       - CI setup: isolated sqlx preparation (uses setup-sqlx.sh)"
	@echo "  make sqlx-setup     - Run setup-sqlx.sh script (temp DB for sqlx metadata)"
	@echo ""
	@echo "🗄️  Database:"
	@echo "  make db-up          - Start development PostgreSQL database"
	@echo "  make db-down        - Stop development database"
	@echo "  make db-reset       - Reset database (drop and recreate)"
	@echo "  make migrate        - Run database migrations"
	@echo ""
	@echo "🔍 SQLx:"
	@echo "  make sqlx-prepare   - Generate SQLx query cache for offline builds"
	@echo "  make sqlx-check     - Verify SQLx query cache is up to date"
	@echo ""
	@echo "🛠️  Development:"
	@echo "  make build          - Build the project (requires DATABASE_URL)"
	@echo "  make build-offline  - Build using offline SQLx cache"
	@echo "  make test           - Run all tests"
	@echo "  make test-unit      - Run unit tests only"
	@echo "  make test-db        - Run tests requiring database"
	@echo "  make run            - Run the server (requires DATABASE_URL)"
	@echo "  make run-dev        - Run with dev database (combines db-up + run)"
	@echo ""
	@echo "✅ Quality:"
	@echo "  make check          - Run all checks (fmt, lint, sqlx-check, test)"
	@echo "  make fmt            - Format code with rustfmt"
	@echo "  make lint           - Run clippy linter"
	@echo "  make clean          - Clean build artifacts"

# Database URL for local development
DB_URL := postgres://postgres:postgres@localhost:5432/bloggen

# Start development database
db-up:
	@echo "Starting development database..."
	docker-compose -f docker-compose.dev.yml up -d
	@echo "Waiting for database to be ready..."
	@sleep 3
	@echo "Database is ready at $(DB_URL)"

# Stop development database
db-down:
	@echo "Stopping development database..."
	docker-compose -f docker-compose.dev.yml down

# Reset database (complete teardown and restart)
db-reset: db-down
	@echo "Removing database volumes..."
	docker-compose -f docker-compose.dev.yml down -v
	@make db-up

# Run database migrations
migrate:
	@echo "Running database migrations..."
	@DATABASE_URL=$(DB_URL) cargo sqlx migrate run
	@echo "Migrations complete"

# Generate SQLx query cache
sqlx-prepare:
	@echo "Generating SQLx query cache..."
	@DATABASE_URL=$(DB_URL) cargo sqlx prepare
	@echo "Query cache generated at .sqlx/"
	@echo "You can now build with SQLX_OFFLINE=true"

# Verify SQLx query cache is up to date
sqlx-check:
	@echo "Checking SQLx query cache..."
	@SQLX_OFFLINE=true cargo sqlx prepare --check

# Build the project
build:
	@echo "Building v2-server..."
	@cargo build

# Run the server
run:
	@echo "Starting server..."
	@echo "Using DATABASE_URL=$(DB_URL)"
	@DATABASE_URL=$(DB_URL) cargo run

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean

# Full setup for new developers
setup: db-up migrate sqlx-prepare
	@echo ""
	@echo "✅ Setup complete!"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Add to your shell profile (~/.bashrc, ~/.zshrc):"
	@echo "     export DATABASE_URL=\"$(DB_URL)\""
	@echo ""
	@echo "  2. Build the project:"
	@echo "     make build"
	@echo ""
	@echo "  3. Run tests:"
	@echo "     make test"
	@echo ""
	@echo "  4. Start the server:"
	@echo "     make run"

# Run setup-sqlx.sh script (creates temp DB, generates .sqlx, cleans up)
sqlx-setup:
	@echo "Running setup-sqlx.sh script..."
	@chmod +x setup-sqlx.sh
	@./setup-sqlx.sh

# CI-friendly setup (uses isolated temp database)
setup-ci: sqlx-setup
	@echo ""
	@echo "✅ CI setup complete!"
	@echo "SQLx metadata has been generated in .sqlx/"
	@echo "You can now run: make build-offline"

# Build with offline SQLx (no database connection required)
build-offline:
	@echo "Building with offline SQLx cache..."
	@SQLX_OFFLINE=true cargo build

# Build and run with offline SQLx
run-offline:
	@echo "Running with offline SQLx cache..."
	@SQLX_OFFLINE=true cargo run

# Run development server (starts DB first if needed)
run-dev:
	@if ! docker ps | grep -q bloggen-dev-db; then \
		echo "Database not running, starting it..."; \
		make db-up; \
	fi
	@make run

# Unit tests (no database required)
test-unit:
	@echo "Running unit tests..."
	@cargo test --lib

# Database integration tests
test-db: db-up migrate
	@echo "Running database integration tests..."
	@DATABASE_URL=$(DB_URL) cargo test --test '*'

# Run all tests with database
test: db-up migrate
	@echo "Running all tests..."
	@DATABASE_URL=$(DB_URL) cargo test

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt

# Check formatting
fmt-check:
	@echo "Checking code formatting..."
	@cargo fmt -- --check

# Run clippy linter
lint:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings

# Run all quality checks
check: fmt-check lint sqlx-check test
	@echo ""
	@echo "✅ All checks passed!"
