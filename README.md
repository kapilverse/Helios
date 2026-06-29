# HELIOS

A real-time collaborative document editor — like Google Docs, but open-source and built from scratch with Rust CRDTs.

Multiple users can open the same document in their browsers, type simultaneously, and see each other's edits appear in real time with live cursors. No operational transforms, no fighting over positions — every character gets a unique ID, so concurrent edits always converge.

## What Is This?

HELIOS is a full-stack collaborative editing system:

- **Backend**: Rust server handling WebSocket connections, conflict resolution (CRDT + OT reconciler), persistence, and authentication
- **Frontend**: React app with a real-time text editor, login screen, and multi-cursor display
- **Protocol**: Custom WebSocket protocol with delta sync — on reconnect, you only get the ops you missed, not the whole document

Two people open the document → both type at the same time → edits merge automatically → no conflicts, no data loss.

## How To Run

### Option 1: Docker (recommended)

```bash
docker compose up --build
```

Open **http://localhost:3000** — enter your name and start typing. Open another browser tab to see real-time sync.

### Option 2: Run manually

**Terminal 1 — Start the Rust server:**
```bash
cargo run --bin helios-server
```

**Terminal 2 — Start the React frontend:**
```bash
cd frontend
npm install
npm run dev
```

Open **http://localhost:5173** — enter your name and start typing. Open another tab to see your edits sync in real time.

### What you'll see

1. **Login screen** — enter your name
2. **Editor** — a dark-themed text editor with a green "Connected" badge
3. **Multi-cursor** — other users' names and cursors appear at the top
4. **Real-time sync** — type in one tab, see it appear in the other instantly

## Architecture

```
Browser (React)  ◄──WebSocket──►  Axum Server (Rust)
                                    ├── OT Reconciler (conflict resolution)
                                    ├── CRDT Engine (convergence guarantee)
                                    ├── Presence (cursors, heartbeat)
                                    └── Storage (op log, snapshots)
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust, Tokio, Axum |
| Frontend | React, TypeScript, Vite |
| CRDT | Custom RGA (Replicated Growable Array) |
| Protocol | WebSocket + JSON (FlatBuffers planned) |
| Storage | In-memory (SQLite/Postgres planned) |
| Auth | JWT with HMAC-SHA256 |
| WASM | wasm-bindgen (ghost state for optimistic updates) |
| Deployment | Docker, GitHub Actions CI/CD |

## Crates

| Crate | What it does |
|-------|-------------|
| `helios-crdt` | Core CRDT — RGA sequence, LWW Map, Op log |
| `helios-ot-reconciler` | Resolves conflicts between concurrent ops |
| `helios-network` | WebSocket server, broadcasts, presence |
| `helios-sync` | Protocol message types |
| `helios-presence` | Cursor tracking, heartbeat cleanup |
| `helios-storage` | Op log persistence, snapshots |
| `helios-auth` | JWT tokens, role-based access |
| `helios-telemetry` | Latency and convergence metrics |
| `helios-wasm-runtime` | Ghost state for browser-side CRDT |
| `helios-bench` | Performance benchmarks |

## Development

```bash
# Run all tests (66 tests)
cargo test --workspace

# Run integration tests (starts server, connects 2 WebSocket clients)
cargo test -p helios-server --test integration

# Run benchmarks
cargo bench --package helios-bench

# Lint and format
cargo clippy --workspace -- -D warnings
cargo fmt --all

# Type-check frontend
cd frontend && npx tsc --noEmit
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
│   ├── src/lib/ws.ts        # WebSocket client
│   ├── src/hooks/useHelios.ts
│   ├── src/components/      # Login, Editor
│   └── src/App.tsx
├── proto/                   # FlatBuffers schemas
├── Dockerfile               # Multi-stage build
├── docker-compose.yml
└── .github/workflows/       # CI/CD
```

## How It Works

**CRDT**: Each character gets a unique ID `(peer_id, clock)`. Operations carry IDs, not positions. Two clients inserting at the same position → deterministic order by peer ID.

**OT Reconciler**: Server receives ops, checks constraints, emits corrections if needed.

**Ghost State**: User types → op applied locally instantly (optimistic) → server may send correction → browser animates the diff.

**Delta Sync**: Every op has a sequence number. On reconnect, client sends `{since: last_seen_seq}` and gets only missed ops.

## Tests

66 tests passing:
- CRDT convergence (fuzz-tested with 100 random orderings)
- OT transform for all conflict scenarios
- WebSocket integration (2-client sync, broadcast, delta sync, presence)
- JWT creation/verification
- Storage snapshot/replay
- Presence heartbeat

## License

MIT
