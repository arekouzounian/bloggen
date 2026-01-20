# BlogGen v2 Server - Production Deployment Guide

This guide covers deploying the BlogGen v2 server with Docker Compose in a production environment.

## Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- At least 2GB RAM available
- 10GB disk space (for database growth)

## Quick Start

### 1. Configure Environment Variables

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env and set a strong password
nano .env
```

**Important:** Change `POSTGRES_PASSWORD` to a strong, unique password before deployment.

### 2. Configure the Server

Edit `config.json` to match your environment:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3000
  },
  "database": {
    "url": "postgres://bloggen:YOUR_PASSWORD@postgres:5432/bloggen",
    "max_connections": 20,
    "min_connections": 5
  },
  "logging": {
    "level": "info",
    "json": true
  }
}
```

**Note:** The database password in `config.json` must match `POSTGRES_PASSWORD` in `.env`.

### 3. Start the Services

```bash
# Build and start all services
docker compose up -d

# Check service health
docker compose ps

# View logs
docker compose logs -f
```

### 4. Verify Deployment

```bash
# Check server health
curl http://localhost:3000/health

# Should return: {"status":"ok"}
```

## Architecture

The deployment consists of two services:

- **postgres**: PostgreSQL 16 database with persistent storage
- **server**: BlogGen v2 Rust server with content-addressable AST storage

### Data Persistence

Two Docker volumes ensure data persists across container restarts:

- `postgres_data`: Database files (tables, indexes, WAL)
- `server_logs`: Application logs in `/var/log/bloggen/`

### Network

Services communicate over a dedicated bridge network (`bloggen-network`), isolated from other containers.

## Configuration

### Server Configuration (config.json)

| Field | Description | Default |
|-------|-------------|---------|
| `server.host` | Bind address | `0.0.0.0` |
| `server.port` | HTTP port | `3000` |
| `database.url` | PostgreSQL connection string | See config.json |
| `database.max_connections` | Max DB connections | `20` |
| `database.min_connections` | Min idle connections | `5` |
| `logging.level` | Log verbosity (trace/debug/info/warn/error) | `info` |
| `logging.json` | Use JSON logging | `true` |

### Environment Variables

Set in `.env` file:

| Variable | Description | Required |
|----------|-------------|----------|
| `POSTGRES_PASSWORD` | Database password | **Yes** |
| `POSTGRES_USER` | Database user | No (default: bloggen) |
| `POSTGRES_DB` | Database name | No (default: bloggen) |
| `POSTGRES_PORT` | Exposed DB port | No (default: 5432) |
| `SERVER_PORT` | Exposed server port | No (default: 3000) |
| `RUST_LOG` | Override log level | No (default: info) |

## Management Commands

### View Logs

```bash
# All services
docker compose logs -f

# Server only
docker compose logs -f server

# Database only
docker compose logs -f postgres

# Last 100 lines
docker compose logs --tail=100
```

### Restart Services

```bash
# Restart all
docker compose restart

# Restart server only
docker compose restart server

# Restart database only (will briefly interrupt service)
docker compose restart postgres
```

### Stop Services

```bash
# Stop without removing containers
docker compose stop

# Stop and remove containers (data persists in volumes)
docker compose down

# Stop and remove everything INCLUDING DATA
docker compose down -v  # ⚠️  WARNING: Deletes all data!
```

### Update Deployment

```bash
# Pull latest changes
git pull

# Rebuild and restart
docker compose up -d --build

# View migration logs
docker compose logs server | grep migration
```

## Database Management

### Access PostgreSQL CLI

```bash
docker compose exec postgres psql -U bloggen -d bloggen
```

### Backup Database

```bash
# Create backup
docker compose exec postgres pg_dump -U bloggen bloggen > backup_$(date +%Y%m%d_%H%M%S).sql

# Or using pg_dumpall for full backup including roles
docker compose exec postgres pg_dumpall -U bloggen > full_backup_$(date +%Y%m%d_%H%M%S).sql
```

### Restore Database

```bash
# Stop server to prevent writes during restore
docker compose stop server

# Restore from backup
docker compose exec -T postgres psql -U bloggen -d bloggen < backup_20260119_120000.sql

# Restart server
docker compose start server
```

### View Database Size

```bash
docker compose exec postgres psql -U bloggen -d bloggen -c "
  SELECT
    pg_size_pretty(pg_database_size('bloggen')) as db_size,
    pg_size_pretty(pg_total_relation_size('ast_nodes')) as ast_nodes_size,
    pg_size_pretty(pg_total_relation_size('posts')) as posts_size;
"
```

## Monitoring

### Health Checks

Both services include built-in health checks:

```bash
# View health status
docker compose ps

# Server health endpoint
curl http://localhost:3000/health
```

### Resource Usage

```bash
# Container stats
docker stats bloggen-server bloggen-postgres

# Disk usage
docker system df -v | grep bloggen
```

### Log Rotation

Server logs are stored in the `server_logs` volume. Configure log rotation:

```bash
# Access log volume
docker run --rm -v bloggen_server_logs:/logs alpine ls -lh /logs

# Manual cleanup of old logs (example: keep last 7 days)
docker run --rm -v bloggen_server_logs:/logs alpine find /logs -name "*.log" -mtime +7 -delete
```

## Security

### Recommended Security Practices

1. **Change default password**: Never use default passwords in production
2. **Firewall rules**: Restrict port 5432 to localhost only
3. **Regular updates**: Keep Docker images updated
4. **Backup encryption**: Encrypt database backups
5. **Network isolation**: Use Docker networks, not host networking
6. **Resource limits**: Configure deploy.resources in docker-compose.yml

### Update PostgreSQL Password

```bash
# 1. Stop the server
docker compose stop server

# 2. Update password in database
docker compose exec postgres psql -U bloggen -c "ALTER USER bloggen WITH PASSWORD 'new_secure_password';"

# 3. Update config.json with new password
nano config.json

# 4. Update .env with new password
nano .env

# 5. Restart all services
docker compose up -d
```

## Troubleshooting

### Server won't start

```bash
# Check logs
docker compose logs server

# Common issues:
# - Database not ready: Wait for postgres healthcheck to pass
# - Config error: Validate config.json syntax
# - Port conflict: Check if port 3000 is already in use
lsof -i :3000
```

### Database connection errors

```bash
# Verify database is running
docker compose ps postgres

# Test connection
docker compose exec postgres psql -U bloggen -d bloggen -c "SELECT 1;"

# Check connection string in config.json matches .env
```

### Migration failures

```bash
# View migration status
docker compose exec postgres psql -U bloggen -d bloggen -c "SELECT * FROM _sqlx_migrations;"

# Manually run migrations
docker compose exec server /app/bloggen-server migrate
```

### Out of disk space

```bash
# Check volume sizes
docker system df -v

# Clean up unused images and containers
docker system prune -a

# Consider adding volume size limits or implementing log rotation
```

## Performance Tuning

### Database Tuning

Edit `postgresql.conf` (requires mounting custom config):

```conf
# Memory
shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
work_mem = 16MB

# Connections
max_connections = 100

# WAL
wal_buffers = 16MB
checkpoint_completion_target = 0.9
```

Mount in docker-compose.yml:

```yaml
volumes:
  - ./postgresql.conf:/etc/postgresql/postgresql.conf
command: postgres -c config_file=/etc/postgresql/postgresql.conf
```

### Server Tuning

Adjust in `config.json`:

- Increase `max_connections` for high traffic (but monitor database load)
- Decrease `min_connections` to reduce idle resource usage
- Set `logging.level` to `warn` in production to reduce I/O

## Scaling

### Horizontal Scaling

To run multiple server instances behind a load balancer:

```yaml
# docker-compose.yml
server:
  # ... existing config ...
  deploy:
    replicas: 3
```

### Vertical Scaling

Increase resource limits:

```yaml
deploy:
  resources:
    limits:
      cpus: '4'
      memory: 2G
```

## Support

For issues or questions:
- Check logs: `docker compose logs -f`
- Review this guide
- File an issue on GitHub
