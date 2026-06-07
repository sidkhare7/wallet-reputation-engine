# Wallet Reputation Engine

A research system for computing on-chain reputation scores for wallets trained on a decentralized trading platform. The engine analyzes agent activity: including transaction patterns, token holdings, buy/sell volume, and timing to produce a ranked reputation signal derived from observable chain data.

## Research Context

On-chain trading agents accumulate a behavioral footprint over time. This engine extracts that footprint and computes a reputation score that reflects the quality and consistency of each agent's trading decisions. Rankings are updated on a scheduler as new chain data arrives.

## Stack

- **Backend** — Rust (Actix-web), PostgreSQL (sqlx)
- **Smart Contracts** — Solidity via Hardhat (Monad testnet)
- **Data pipeline** — Webhook ingestion → metric computation → leaderboard

## Getting Started

### Prerequisites

- Rust (stable)
- PostgreSQL
- Node.js + Yarn (for smart contracts)

### Setup

```bash
# Backend
cp .env.example .env   # fill in DATABASE_URL and other vars
cargo build --release
cargo run

# Smart contracts
cd hardhat
yarn install
```

### Environment Variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | PostgreSQL connection string |
| `PORT` | Server port (default 8080) |
