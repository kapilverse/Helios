# HELIOS

Real-time collaborative document editing powered by CRDTs. Two browsers, one document, live cursors, visible conflict resolution.

## Architecture

```
┌─────────────┐     WebSocket      ┌──────────────┐
│   Browser    │ ◄───────────────► │  Axum Server  │
│  (React +   │    FlatBuffers     │  ┌─────────┐  │
│   WASM)     │                    │  │ OT Recon │  │
└─────────────┘                    │  └─────────┘  │
                                   │  ┌─────────┐  │
┌─────────────┐     WebSocket      │  │ CRDT    │  │
│   Browser    │ ◄───────────────► │  │ Engine  │  │
│  (React +   │                    │  └─────────┘  │
│   WASM)     │                    └──────────────┘
└─────────────┘                           │
                                    ┌─────┴─────┐
                                    │  Storage   │
                                    │  (SQLite)  │
                                    └───────────┘
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Language | Rust (2024 edition) |
| Async runtime | Tokio |
| HTTP/WebSocket | Axum |
| Serialization | FlatBuffers |
| Storage | SQLite (dev) / Postgres (prod) |
| WASM | wasm-bindgen + wasm-pack |
| Frontend | React + TypeScript + Vite |
| Metrics | OpenTelemetry |
| CI/CD | GitHub Actions + Docker |

## Crates

| Crate | Purpose |
|-------|---------|
| `helios-crdt` | RGA sequence CRDT, LWW Map, Op log |
| `helios-ot-reconciler` | Transform function, conflict resolution |
| `helios-network` | WebSocket server, delta sync, presence |
| `helios-sync` | Protocol types, sync state |
| `helios-presence` | Heartbeat, selection, viewport tracking |
| `helios-storage` | Op log, snapshots, tail replay |
| `helios-auth` | JWT, role-based access control |
| `helios-telemetry` | Latency, convergence metrics |
| `helios-wasm-runtime` | Ghost state, optimistic updates |
| `helios-bench` | Criterion benchmarks |
| `helios-server` | Binary + integration tests |

## Quick Start

### Prerequisites

- Rust 1.78+
- Node.js 22+
- Docker (optional)

### Run locally

```bash
# Start the server
cargo run --bin helios-server

# In another terminal, start the frontend
cd frontend && npm install && npm run dev
```

Open `http://localhost:5173` — login, type, open another tab to see real-time sync.

### Run with Docker

```bash
docker compose up --build
```

Open `http://localhost:3000`.

## Development

### Run all tests

```bash
cargo test --workspace
```

### Run integration tests

```bash
# Start server first
cargo run --bin helios-server &

# Run integration tests
cargo test -p helios-server --test integration
```

### Run benchmarks

```bash
cargo bench --package helios-bench
```

### Lint and format

```bash
cargo clippy --workspace -- -D warnings
cargo fmt --all
```

## Project Structure

```
helios/
├── crdt/                    # Core CRDT implementation
├── ot-reconciler/           # OT transform function
├── network/                 # WebSocket server
├── sync/                    # Protocol types
├── presence/                # Cursor/selection tracking
├── storage/                 # Op log + snapshots
├── auth/                    # JWT authentication
├── telemetry/               # Metrics collection
├── wasm-runtime/            # WASM client runtime
├── bench/                   # Benchmarks
├── server/                  # Server binary + integration tests
├── frontend/                # React frontend
├── proto/                   # FlatBuffers schemas
├── Dockerfile               # Multi-stage build
├── docker-compose.yml       # Local deployment
└── .github/workflows/       # CI/CD
```

## How It Works

### CRDT (Conflict-free Replicated Data Type)

Each character gets a unique ID `(peer_id, clock)`. Operations carry IDs, not positions. When two clients insert at the same position, ordering is deterministic by peer ID.

### OT Reconciler

Before broadcasting, the server checks semantic constraints:
- Table cell: last writer wins
- Tree node: one parent only
- Character ordering: deterministic by peer ID

### Ghost State (WASM)

```
User types → op generated → CRDT applied locally (optimistic)
Op sent to server → server may emit correction
If correction → diff optimistic vs reconciled → animate morph
If no correction → confirm optimistic state
```

### Delta Sync

- Every op has a sequence number on the server
- Clients track `last_seen_seq`
- On reconnect: client sends `{since: last_seen_seq}` and gets only missed ops

## API

### WebSocket Messages

**Client → Server:**
```json
{"Join": {"document_id": "default"}}
{"Op": {"op": {"Insert": {"id": {"peer": "...", "clock": 1}, "after": null, "content": "H"}}}}
{"Presence": {"cursor": {"peer": "...", "clock": 1}}}
```

**Server → Client:**
```json
{"Sync": {"response": {"ops": [], "current_seq": 42}}}
{"Op": {"op": {...}, "seq": 42}}
{"Presence": [{"name": "Alice", "color": "#ff0000", "op_id": {...}}]}
```

## Tests

60+ tests covering:
- CRDT convergence (fuzz-tested with 100 random orderings)
- OT transform for all conflict scenarios
- WebSocket integration (2-client sync, broadcast, delta sync, presence)
- JWT creation/verification
- Storage snapshot/replay
- Presence heartbeat

## License

MIT
