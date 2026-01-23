#!/usr/bin/env bash
#
# Unified Test Runner for BlogGen
#
# This script provides a unified interface to run all tests:
# - Unit tests (client and server in parallel)
# - Integration tests (e2e-client, e2e-full, e2e-fuse)
#
# Usage:
#   ./run-tests.sh [OPTIONS]
#
# Options:
#   --unit           Run only unit tests (default: run all)
#   --integration    Run only integration tests (default: run all)
#   --e2e-client     Run only client e2e tests
#   --e2e-full       Run only full e2e tests
#   --e2e-fuse       Run only fuse e2e tests
#   --parallel       Run unit tests in parallel (default)
#   --sequential     Run unit tests sequentially
#   --verbose        Show verbose output
#   --help           Show this help message
#
# Environment Variables:
#   SKIP_DB_SETUP=1       Skip database setup for integration tests
#   KEEP_DB_RUNNING=1     Keep database running after tests
#   SERVER_PORT=3000      Server port for integration tests
#

set -e  # Exit on error
set -u  # Exit on undefined variable

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
RUN_UNIT=false
RUN_INTEGRATION=false
RUN_E2E_CLIENT=false
RUN_E2E_FULL=false
RUN_E2E_FUSE=false
PARALLEL=true
VERBOSE=false

# Test results tracking
UNIT_TESTS_PASSED=false
INTEGRATION_TESTS_PASSED=false
E2E_CLIENT_PASSED=false
E2E_FULL_PASSED=false
E2E_FUSE_PASSED=false

# Helper functions
print_header() {
    echo ""
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${BLUE}  $1${NC}"
    echo -e "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_section() {
    echo ""
    echo -e "${CYAN}▸ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

show_help() {
    cat << EOF
Unified Test Runner for BlogGen

Usage: $0 [OPTIONS]

Options:
  --unit           Run only unit tests (default: run all)
  --integration    Run only integration tests (default: run all)
  --e2e-client     Run only client e2e tests
  --e2e-full       Run only full e2e tests
  --e2e-fuse       Run only fuse e2e tests
  --parallel       Run unit tests in parallel (default)
  --sequential     Run unit tests sequentially
  --verbose        Show verbose output
  --help           Show this help message

Examples:
  # Run all tests
  $0

  # Run only unit tests
  $0 --unit

  # Run only integration tests
  $0 --integration

  # Run specific e2e test
  $0 --e2e-client

  # Run unit tests sequentially with verbose output
  $0 --unit --sequential --verbose

Environment Variables:
  SKIP_DB_SETUP=1       Skip database setup for integration tests
  KEEP_DB_RUNNING=1     Keep database running after tests
  SERVER_PORT=3000      Server port for integration tests
EOF
    exit 0
}

# Parse command line arguments
if [ $# -eq 0 ]; then
    # No arguments: run all tests
    RUN_UNIT=true
    RUN_INTEGRATION=true
else
    while [ $# -gt 0 ]; do
        case "$1" in
            --unit)
                RUN_UNIT=true
                shift
                ;;
            --integration)
                RUN_INTEGRATION=true
                shift
                ;;
            --e2e-client)
                RUN_E2E_CLIENT=true
                shift
                ;;
            --e2e-full)
                RUN_E2E_FULL=true
                shift
                ;;
            --e2e-fuse)
                RUN_E2E_FUSE=true
                shift
                ;;
            --parallel)
                PARALLEL=true
                shift
                ;;
            --sequential)
                PARALLEL=false
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --help|-h)
                show_help
                ;;
            *)
                echo -e "${RED}Error: Unknown option: $1${NC}"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
fi

# If specific e2e tests are selected, enable integration
if [ "$RUN_E2E_CLIENT" = true ] || [ "$RUN_E2E_FULL" = true ] || [ "$RUN_E2E_FUSE" = true ]; then
    RUN_INTEGRATION=true
fi

# Check if we're in the bloggen root directory
if [ ! -d "client/bgc" ] || [ ! -d "v2-server" ]; then
    echo -e "${RED}Error: Must run from bloggen repository root${NC}"
    echo "Expected directory structure:"
    echo "  ./client/bgc/     - BlogGen client"
    echo "  ./v2-server/      - BlogGen v2 server"
    exit 1
fi

REPO_ROOT="$(pwd)"

# ============================================================================
# Print Test Configuration
# ============================================================================

print_header "BlogGen Unified Test Suite"
echo ""
echo "Configuration:"
if [ "$RUN_UNIT" = true ]; then
    echo -e "  ${GREEN}●${NC} Unit tests: enabled"
    if [ "$PARALLEL" = true ]; then
        echo "    - Execution: parallel"
    else
        echo "    - Execution: sequential"
    fi
else
    echo -e "  ${YELLOW}○${NC} Unit tests: disabled"
fi

if [ "$RUN_INTEGRATION" = true ]; then
    echo -e "  ${GREEN}●${NC} Integration tests: enabled"
    if [ "$RUN_E2E_CLIENT" = false ] && [ "$RUN_E2E_FULL" = false ] && [ "$RUN_E2E_FUSE" = false ]; then
        echo "    - All integration tests will run"
    else
        [ "$RUN_E2E_CLIENT" = true ] && echo "    - e2e-client"
        [ "$RUN_E2E_FULL" = true ] && echo "    - e2e-full"
        [ "$RUN_E2E_FUSE" = true ] && echo "    - e2e-fuse"
    fi
else
    echo -e "  ${YELLOW}○${NC} Integration tests: disabled"
fi

if [ "$VERBOSE" = true ]; then
    echo -e "  ${GREEN}●${NC} Verbose output: enabled"
fi

# ============================================================================
# Run Unit Tests
# ============================================================================

if [ "$RUN_UNIT" = true ]; then
    print_header "Phase 1: Unit Tests"
    
    if [ "$PARALLEL" = true ]; then
        print_section "Running client and server unit tests in parallel..."
        
        # Create temp directory for logs
        TEST_LOGS=$(mktemp -d)
        trap "rm -rf $TEST_LOGS" EXIT
        
        # Run client tests in background
        (
            cd "$REPO_ROOT/client/bgc"
            if [ "$VERBOSE" = true ]; then
                cargo test 2>&1 | tee "$TEST_LOGS/client.log"
            else
                cargo test > "$TEST_LOGS/client.log" 2>&1
            fi
            echo $? > "$TEST_LOGS/client.exit"
        ) &
        CLIENT_PID=$!
        
        # Run server tests in background
        (
            cd "$REPO_ROOT/v2-server"
            if [ "$VERBOSE" = true ]; then
                cargo test 2>&1 | tee "$TEST_LOGS/server.log"
            else
                cargo test > "$TEST_LOGS/server.log" 2>&1
            fi
            echo $? > "$TEST_LOGS/server.exit"
        ) &
        SERVER_PID=$!
        
        # Wait for both to complete
        wait $CLIENT_PID
        wait $SERVER_PID
        
        # Check results
        CLIENT_EXIT=$(cat "$TEST_LOGS/client.exit")
        SERVER_EXIT=$(cat "$TEST_LOGS/server.exit")
        
        echo ""
        if [ "$CLIENT_EXIT" -eq 0 ]; then
            CLIENT_TESTS=$(grep "test result:" "$TEST_LOGS/client.log" | head -1 | sed 's/test result: ok. //' | sed 's/ passed;.*//')
            print_success "Client tests passed ($CLIENT_TESTS tests)"
        else
            print_error "Client tests failed"
            if [ "$VERBOSE" = false ]; then
                echo "Client test output:"
                tail -50 "$TEST_LOGS/client.log"
            fi
        fi
        
        if [ "$SERVER_EXIT" -eq 0 ]; then
            SERVER_TESTS=$(grep "test result:" "$TEST_LOGS/server.log" | head -1 | sed 's/test result: ok. //' | sed 's/ passed;.*//')
            print_success "Server tests passed ($SERVER_TESTS tests)"
        else
            print_error "Server tests failed"
            if [ "$VERBOSE" = false ]; then
                echo "Server test output:"
                tail -50 "$TEST_LOGS/server.log"
            fi
        fi
        
        if [ "$CLIENT_EXIT" -eq 0 ] && [ "$SERVER_EXIT" -eq 0 ]; then
            UNIT_TESTS_PASSED=true
        else
            UNIT_TESTS_PASSED=false
        fi
    else
        # Sequential execution
        print_section "Running client unit tests..."
        cd "$REPO_ROOT/client/bgc"
        if [ "$VERBOSE" = true ]; then
            cargo test
        else
            cargo test --quiet
        fi
        CLIENT_EXIT=$?
        
        if [ "$CLIENT_EXIT" -eq 0 ]; then
            print_success "Client tests passed"
        else
            print_error "Client tests failed"
            exit 1
        fi
        
        print_section "Running server unit tests..."
        cd "$REPO_ROOT/v2-server"
        if [ "$VERBOSE" = true ]; then
            cargo test
        else
            cargo test --quiet
        fi
        SERVER_EXIT=$?
        
        if [ "$SERVER_EXIT" -eq 0 ]; then
            print_success "Server tests passed"
            UNIT_TESTS_PASSED=true
        else
            print_error "Server tests failed"
            UNIT_TESTS_PASSED=false
            exit 1
        fi
    fi
fi

# ============================================================================
# Run Integration Tests
# ============================================================================

if [ "$RUN_INTEGRATION" = true ]; then
    print_header "Phase 2: Integration Tests"
    
    cd "$REPO_ROOT"
    
    # Determine which integration tests to run
    RUN_ALL_E2E=false
    if [ "$RUN_E2E_CLIENT" = false ] && [ "$RUN_E2E_FULL" = false ] && [ "$RUN_E2E_FUSE" = false ]; then
        RUN_ALL_E2E=true
    fi
    
    # Run e2e-client
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_CLIENT" = true ]; then
        print_section "Running client e2e tests..."
        if [ "$VERBOSE" = true ]; then
            ./tst/e2e-client.sh
        else
            ./tst/e2e-client.sh > /dev/null 2>&1
        fi
        
        if [ $? -eq 0 ]; then
            print_success "Client e2e tests passed"
            E2E_CLIENT_PASSED=true
        else
            print_error "Client e2e tests failed"
            E2E_CLIENT_PASSED=false
            if [ "$RUN_ALL_E2E" = false ]; then
                exit 1
            fi
        fi
    fi
    
    # Run e2e-full
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_FULL" = true ]; then
        print_section "Running full integration tests..."
        if [ "$VERBOSE" = true ]; then
            ./tst/e2e-full.sh
        else
            ./tst/e2e-full.sh > /dev/null 2>&1
        fi
        
        if [ $? -eq 0 ]; then
            print_success "Full integration tests passed"
            E2E_FULL_PASSED=true
        else
            print_error "Full integration tests failed"
            E2E_FULL_PASSED=false
            if [ "$RUN_ALL_E2E" = false ]; then
                exit 1
            fi
        fi
    fi
    
    # Run e2e-fuse
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_FUSE" = true ]; then
        print_section "Running FUSE integration tests..."
        if [ "$VERBOSE" = true ]; then
            ./tst/e2e-fuse.sh
        else
            ./tst/e2e-fuse.sh > /dev/null 2>&1
        fi
        
        if [ $? -eq 0 ]; then
            print_success "FUSE integration tests passed"
            E2E_FUSE_PASSED=true
        else
            print_error "FUSE integration tests failed"
            E2E_FUSE_PASSED=false
            if [ "$RUN_ALL_E2E" = false ]; then
                exit 1
            fi
        fi
    fi
    
    # Check if all requested integration tests passed
    INTEGRATION_TESTS_PASSED=true
    if [ "$RUN_ALL_E2E" = true ]; then
        if [ "$E2E_CLIENT_PASSED" = false ] || [ "$E2E_FULL_PASSED" = false ] || [ "$E2E_FUSE_PASSED" = false ]; then
            INTEGRATION_TESTS_PASSED=false
        fi
    else
        if [ "$RUN_E2E_CLIENT" = true ] && [ "$E2E_CLIENT_PASSED" = false ]; then
            INTEGRATION_TESTS_PASSED=false
        fi
        if [ "$RUN_E2E_FULL" = true ] && [ "$E2E_FULL_PASSED" = false ]; then
            INTEGRATION_TESTS_PASSED=false
        fi
        if [ "$RUN_E2E_FUSE" = true ] && [ "$E2E_FUSE_PASSED" = false ]; then
            INTEGRATION_TESTS_PASSED=false
        fi
    fi
fi

# ============================================================================
# Print Final Summary
# ============================================================================

print_header "Test Summary"
echo ""

TOTAL_FAILED=0

if [ "$RUN_UNIT" = true ]; then
    if [ "$UNIT_TESTS_PASSED" = true ]; then
        print_success "Unit tests: PASSED"
    else
        print_error "Unit tests: FAILED"
        TOTAL_FAILED=$((TOTAL_FAILED + 1))
    fi
fi

if [ "$RUN_INTEGRATION" = true ]; then
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_CLIENT" = true ]; then
        if [ "$E2E_CLIENT_PASSED" = true ]; then
            print_success "Client e2e tests: PASSED"
        else
            print_error "Client e2e tests: FAILED"
            TOTAL_FAILED=$((TOTAL_FAILED + 1))
        fi
    fi
    
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_FULL" = true ]; then
        if [ "$E2E_FULL_PASSED" = true ]; then
            print_success "Full integration tests: PASSED"
        else
            print_error "Full integration tests: FAILED"
            TOTAL_FAILED=$((TOTAL_FAILED + 1))
        fi
    fi
    
    if [ "$RUN_ALL_E2E" = true ] || [ "$RUN_E2E_FUSE" = true ]; then
        if [ "$E2E_FUSE_PASSED" = true ]; then
            print_success "FUSE integration tests: PASSED"
        else
            print_error "FUSE integration tests: FAILED"
            TOTAL_FAILED=$((TOTAL_FAILED + 1))
        fi
    fi
fi

echo ""
if [ "$TOTAL_FAILED" -eq 0 ]; then
    echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${GREEN}  ✓ All tests passed!${NC}"
    echo -e "${BOLD}${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 0
else
    echo -e "${BOLD}${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${RED}  ✗ $TOTAL_FAILED test suite(s) failed${NC}"
    echo -e "${BOLD}${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 1
fi
