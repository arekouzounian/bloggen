.PHONY: help test test-unit test-integration test-all test-client test-server test-e2e-client test-e2e-full test-e2e-fuse clean build

# Default target
help:
	@echo "BlogGen Test Targets"
	@echo "===================="
	@echo ""
	@echo "Testing:"
	@echo "  make test              - Run all tests (unit + integration)"
	@echo "  make test-unit         - Run only unit tests (parallel)"
	@echo "  make test-integration  - Run only integration tests"
	@echo "  make test-client       - Run only client unit tests"
	@echo "  make test-server       - Run only server unit tests"
	@echo "  make test-e2e-client   - Run only client e2e tests"
	@echo "  make test-e2e-full     - Run only full integration tests"
	@echo "  make test-e2e-fuse     - Run only FUSE integration tests"
	@echo ""
	@echo "Building:"
	@echo "  make build             - Build all components"
	@echo "  make build-client      - Build client only"
	@echo "  make build-server      - Build server only"
	@echo ""
	@echo "Cleaning:"
	@echo "  make clean             - Clean all build artifacts"
	@echo ""
	@echo "Advanced:"
	@echo "  make test-verbose      - Run all tests with verbose output"
	@echo "  make test-sequential   - Run unit tests sequentially"
	@echo ""

# Run all tests
test: test-all

test-all:
	@./run-tests.sh

# Run only unit tests (parallel by default)
test-unit:
	@./run-tests.sh --unit

# Run only integration tests
test-integration:
	@./run-tests.sh --integration

# Run specific test suites
test-client:
	@cd client/bgc && cargo test

test-server:
	@cd server && cargo test

test-e2e-client:
	@./run-tests.sh --e2e-client

test-e2e-full:
	@./run-tests.sh --e2e-full

test-e2e-fuse:
	@./run-tests.sh --e2e-fuse

# Advanced test options
test-verbose:
	@./run-tests.sh --verbose

test-sequential:
	@./run-tests.sh --unit --sequential

# Build targets
build:
	@cargo build --workspace

build-client:
	@cd client/bgc && cargo build

build-server:
	@cd server && cargo build

# Clean
clean:
	@cargo clean
