
# Questions

### 1. Docker Deployment & API Downtime

I have a Docker-run backend setup with GitHub Actions. What happens when I push to the repo and it rebuilds the container? How do I ensure my APIs don't go down?
Right now, I'm thinking of building the new container, then—when that's done—switching traffic to the new one and closing the old one. Is containerization a good solution?

---

### 2. Current Stack Overview

**Architecture Flow:**

- Smart contracts deployed to Monad emit events →  
- Goldsky mirrors them →  
- Sends webhook requests to the Rust backend →  
- Backend adds each request to a queue and responds within 5ms (non-blocking),  
- Requests are processed and saved to the DB in ~100–500ms.

**Backend:**

- Rust + TimescaleDB (Postgres-based)
- Admin API handler for running daily cron jobs (e.g., token stats updates, reputation features)
- REST APIs for frontend
- WebSocket for real-time trade data (used in TradingView chart)

**Frontend:**

- Built with Next.js, Tailwind, and TanStack Query
- Polls homepage data every 4 seconds
- On token page: initial request fetches historical data; real-time data is streamed via WebSocket

### 3. Deployment
I have deployed backend to a droplet in digital ocean and created CI/CD pipeline with github actions and docker
I have not added NGNIX and other devops stuff, any recommendations there?

Also I do not know for my threading, how many workers and threads should I have what CPU should I get etc

---

### 4. Security Measures

Currently, we don't have proper security in place.

- **Admin routes**: Protected via a secret header in the request
- **Public APIs**: Planning to just use Cloudflare but I think we would need more security?

Would appreciate suggestions for better security measures.

---
