#!/bin/bash
set -e

# Start a temporary PostgreSQL container
echo "Starting temporary PostgreSQL container..."
docker run --name bloggen-sqlx-temp \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=bloggen \
  -p 5432:5432 \
  -d postgres:16-alpine

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
sleep 3

# Set DATABASE_URL
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/bloggen"

# Run migrations
echo "Running migrations..."
cargo sqlx migrate run

# Generate sqlx metadata
echo "Generating sqlx query metadata..."
cargo sqlx prepare

# Stop and remove container
echo "Cleaning up..."
docker stop bloggen-sqlx-temp
docker rm bloggen-sqlx-temp

echo "Done! The .sqlx directory has been created."
echo "You can now use compile-time checked queries."
