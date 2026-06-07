# Cult Backend - Architecture & Scaling Documentation

## System Overview

The Cult Backend is a Rust-based web service built with Actix-Web that handles cryptocurrency token events, user accounts, and real-time data processing. It serves as the backend for a token trading platform with webhook processing, WebSocket support, and comprehensive API endpoints.

## Current Architecture

### Core Components

#### 1. Web Server (Actix-Web)
- **Framework**: Actix-Web 4.4.0
- **Workers**: 8 worker threads
- **Binding**: Configurable via `SERVER_ADDRESS` (default: 127.0.0.1:8080)
- **Middleware Stack**:
  - CORS (localhost:3000 allowed)
  - Request logging
  - Path normalization
  - Custom authentication (ApiGuard)

#### 2. Database Layer
- **Database**: PostgreSQL with SQLx
- **Connection Pool**: 50 max connections
- **Timeout**: 10 seconds acquire timeout
- **Features**: Async, transaction support, BigDecimal handling

#### 3. Event Processing System
- **Architecture**: Async event queue with background processing
- **Queue Type**: Tokio MPSC channel
- **Buffer Size**: 10,000 events
- **Concurrency**: 100 concurrent jobs
- **Event Broadcasting**: 1,000 event broadcast channel for WebSockets

#### 4. Background Services
- **Price Fetcher**: Periodic ETH price updates via Alchemy API
- **Event Processor**: Handles webhook events asynchronously
- **WebSocket Handler**: Real-time event broadcasting

## API Architecture

### Route Structure

```
/api/                    # Public API endpoints (no auth)
├── account/             # Account management
├── cult/               # Token data and trading
├── diamond_hands/      # Diamond hands leaderboard
├── airdrop/            # Merkle proof endpoints
└── prices/             # Cryptocurrency prices

/admin/                 # Protected admin endpoints (ApiGuard)
├── run-diamond-hands/  # Execute diamond hands algorithm
├── run-community-airdrops/ # Update community data
├── run-community-scores/   # Recalculate scores
└── run-token-stats/    # Update token statistics

/webhook/               # Webhook endpoints (content-type guarded)
├── create-token/       # Token creation events
├── buy-token/          # Token purchase events
├── sell-token/         # Token sale events
├── claim-token/        # Token claim events
└── graduate-token/     # Market graduation events

/ws                     # WebSocket endpoint
```

### Authentication & Security

#### ApiGuard Middleware
```rust
// Environment Variables Required:
ALLOWED_IPS=127.0.0.1,10.0.0.1    # Comma-separated IP whitelist
Upstash-Flow-Control-Key=key1,key2,key3            # Comma-separated API keys

// Headers Required:
X-API-KEY: your-api-key-here
```

**Security Features**:
- IP whitelist validation
- API key authentication
- Applied only to `/admin/*` endpoints
- Regex-based IP validation

#### Webhook Security
- Content-Type guards (`application/json` required)
- Webhook secret validation (configured but commented out)
- Event ID tracking for deduplication

## Event Processing Architecture

### Event Flow
```
Webhook Request → Transformation → Queue → Background Processing → Database → WebSocket Broadcast
```

### Event Types & Transformations

#### 1. Token Creation (`/webhook/create-token`)
```rust
// Input: Array of token creation events
// Output: Normalized token data
{
    "token_address": "0x...",
    "token_creator": "0x...",
    "name": "Token Name",
    "symbol": "TKN",
    "chain_id": "10143",
    // ... additional fields
}
```

#### 2. Token Trading (`/webhook/buy-token`, `/webhook/sell-token`)
```rust
// Input: Hasura webhook format
// Output: Normalized trade data
{
    "trader_id": "0x...",
    "token_id": "0x...",
    "eth_amount": "1000000000000000000",
    "token_amount": "1000000000000000000",
    "chain_id": 10143,
    // ... additional fields
}
```

### Error Handling & Resilience

#### Transaction Management
- Each event processed in isolated transaction
- Automatic rollback on processing failure
- Commit only on successful completion

#### Error Monitoring
- **Sentry Integration**: Automatic error capture and reporting
- **Structured Logging**: Tracing with configurable levels
- **Error Classification**: Different HTTP status codes for different error types

## Current Capacity & Limitations

### Theoretical Capacity
- **Queue Buffer**: 10,000 events
- **Concurrent Processing**: 100 jobs
- **Database Connections**: 50
- **Total Events in Flight**: ~10,100

### Performance Characteristics
- **Sustained Load**: ~500-1,000 events/second
- **Burst Capacity**: ~2,000 events/second (short bursts)
- **Database Bottleneck**: Connection pool is primary constraint
- **Memory Usage**: ~100-200MB baseline

### Known Issues & Error Patterns

#### 1. "Pool timed out" Errors
**Cause**: Database connection pool exhaustion
**Symptoms**: 
- HTTP 503 responses
- Long request latencies
- Queue backups

**Current Mitigation**:
- 10-second acquire timeout
- Transaction-scoped connections
- Automatic connection cleanup

#### 2. "Queue full" Errors  
**Cause**: Event processing slower than ingestion
**Symptoms**:
- HTTP 503 on webhook endpoints
- Event loss
- Memory pressure

**Current Mitigation**:
- 10,000 event buffer
- Async processing
- Error logging and monitoring

#### 3. "Token does not exist" Errors
**Cause**: Race condition between token creation and trading events
**Symptoms**:
- Failed buy/sell event processing
- Data inconsistency
- Transaction rollbacks

**Current Mitigation**:
- Event ordering (creation before trades)
- Transaction isolation
- Retry logic (manual)

## Environment Configuration

### Required Environment Variables
```bash
# Database
DATABASE_URL=postgresql://user:pass@host:port/db

# Server
SERVER_ADDRESS=127.0.0.1:8080          # Optional, defaults shown
RUST_LOG=info                          # Logging level

# External Services
WEBHOOK_SECRET=your-webhook-secret      # Webhook validation
ALCHEMY_API_KEY=your-alchemy-key      # Price fetching

# Security (Admin endpoints)
ALLOWED_IPS=127.0.0.1,10.0.0.1        # IP whitelist
Upstash-Flow-Control-Key=admin-key-1,admin-key-2       # API keys
```

### Optional Configuration
```bash
# Performance Tuning (not currently configurable)
WEBHOOK_MAX_CONCURRENT_JOBS=100        # Event processing concurrency
WEBHOOK_QUEUE_BUFFER=10000            # Event queue size
DATABASE_MAX_CONNECTIONS=50           # DB pool size
DATABASE_ACQUIRE_TIMEOUT=10           # Connection timeout (seconds)
```

## Scaling Strategies

### Immediate Improvements (Low Risk)

#### 1. Database Connection Pool Scaling
```rust
let pool = PgPoolOptions::new()
    .max_connections(100)              // 2x increase
    .min_connections(10)               // Keep warm connections
    .acquire_timeout(Duration::from_secs(30))  // 3x timeout
    .idle_timeout(Duration::from_secs(300))    // 5 min idle
    .max_lifetime(Duration::from_secs(1800))   // 30 min max
```

**Expected Impact**: 2x capacity increase
**Risk**: Low (just configuration)
**Monitoring**: Watch connection utilization

#### 2. Event Queue Scaling
```rust
let webhook_config = models::WebhookConfig {
    max_concurrent_jobs: 200,          // 2x increase
    job_queue_buffer: 25_000,          // 2.5x increase
};
```

**Expected Impact**: 2.5x burst capacity
**Risk**: Medium (memory usage increase)
**Monitoring**: Queue depth, memory usage

#### 3. Worker Thread Scaling
```rust
HttpServer::new(move || { /* ... */ })
    .workers(16)                       // 2x increase
    .bind(server_address)?
```

**Expected Impact**: Better request handling
**Risk**: Low (CPU dependent)
**Monitoring**: CPU utilization, response times

### Medium-Term Improvements (Medium Risk)

#### 1. Event Priority System
```rust
#[derive(Debug, Clone)]
pub struct WebhookPayload {
    pub priority: u8,                  // 0 = highest priority
    pub retry_count: u8,
    pub max_retries: u8,
    // ... existing fields
}
```

**Benefits**: 
- Token creation events processed first
- Reduces race conditions
- Better error recovery

#### 2. Database Read Replicas
- Route read queries to replicas
- Keep writes on primary
- Separate connection pools

#### 3. Caching Layer (Redis)
- Cache frequently accessed token data
- Cache user sessions
- Reduce database load

### Long-Term Scaling (High Risk)

#### 1. Microservices Architecture
- Separate services for each event type
- Independent scaling
- Service mesh (Istio/Linkerd)

#### 2. Event Streaming (Kafka/Redis Streams)
- Replace in-memory queues
- Persistent event storage
- Multi-consumer support

#### 3. Horizontal Scaling
- Load balancer with multiple instances
- Shared database cluster
- Container orchestration (Kubernetes)

## Monitoring & Observability

### Current Monitoring
- **Sentry**: Error tracking and alerting
- **Tracing**: Structured logging with levels
- **Actix Logging**: HTTP request/response logging

### Recommended Metrics
```rust
// Add these metrics endpoints
/metrics                               # Prometheus metrics
├── webhook_events_total              # Counter by event type
├── webhook_queue_depth               # Current queue size
├── database_connections_active       # Active connections
├── database_connections_idle         # Idle connections
├── event_processing_duration         # Histogram
└── error_rate_by_endpoint           # Error percentage
```

### Alert Thresholds
- Queue depth > 8,000 events (80% of buffer)
- Database connections > 40 (80% of pool)
- Error rate > 5%
- Response time > 5 seconds (95th percentile)

## Deployment Architecture

### Current Deployment
- Single server instance
- Direct database connection
- No load balancing
- Manual scaling

### Recommended Production Setup
```yaml
# Docker Compose Example
services:
  cult-backend:
    image: cult-backend:latest
    replicas: 3
    environment:
      - DATABASE_URL=postgresql://...
      - REDIS_URL=redis://...
    resources:
      limits:
        memory: 1GB
        cpu: 1000m
      requests:
        memory: 512MB
        cpu: 500m
  
  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=cult
    volumes:
      - postgres_data:/var/lib/postgresql/data
  
  redis:
    image: redis:7-alpine
    
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
```

## Development Guidelines

### Code Organization
```
src/
├── main.rs                    # Server setup and routing
├── models.rs                  # Data structures and types
├── routes.rs                  # API endpoint handlers
├── handlers/                  # Event processing logic
├── auth/                      # Authentication middleware
├── utils/                     # Utility functions
├── websocket.rs              # WebSocket handling
├── fetch_price.rs            # Price fetching service
├── diamond_hands/            # Diamond hands algorithm
└── community_airdrops/       # Community management
```

### Testing Strategy
- Unit tests for transformation functions
- Integration tests for API endpoints
- Load testing for webhook endpoints
- Database transaction testing

### Performance Best Practices
1. **Database**: Use transactions, avoid N+1 queries
2. **Memory**: Stream large responses, limit query results
3. **Concurrency**: Use async/await, avoid blocking operations
4. **Caching**: Cache expensive computations, use appropriate TTLs
5. **Monitoring**: Log performance metrics, set up alerts

## Troubleshooting Guide

### Common Issues

#### High Memory Usage
```bash
# Check queue depth
curl http://localhost:8080/metrics | grep queue_depth

# Check database connections
curl http://localhost:8080/metrics | grep db_connections

# Monitor with htop/ps
ps aux | grep cult-backend
```

#### Database Connection Issues
```bash
# Check PostgreSQL connections
SELECT count(*) FROM pg_stat_activity WHERE state = 'active';

# Check connection pool settings
SELECT name, setting FROM pg_settings WHERE name LIKE '%connection%';
```

#### Event Processing Delays
```bash
# Check queue status
curl -H "X-API-KEY: your-key" http://localhost:8080/admin/queue-status

# Check recent errors in logs
tail -f /var/log/cult-backend.log | grep ERROR
```

### Performance Tuning Checklist

- [ ] Database connection pool sized appropriately
- [ ] Event queue buffer sized for peak load
- [ ] Worker threads match CPU cores
- [ ] Database indexes on frequently queried columns
- [ ] Monitoring and alerting configured
- [ ] Error handling covers all failure modes
- [ ] Graceful shutdown implemented
- [ ] Health check endpoints available

This architecture supports the current load but will need scaling improvements for high-volume production use. The event processing system is the primary bottleneck and should be the focus of scaling efforts. 