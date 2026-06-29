# Helios (DOCSYNC) Project Structure

This document outlines the high-level architecture and file structure of the DOCSYNC/Helios collaborative editing platform. The project is split into a modular Rust workspace for the backend and a Vite+React frontend.

## 📂 Root Directory

- **`Cargo.toml` / `Cargo.lock`**: The Rust workspace configuration. It ties all the modular backend crates together.
- **`Dockerfile`**: A multi-stage Docker build that compiles both the Vite frontend and the Rust backend into a single unified container for easy deployment (e.g., Render, Fly.io).
- **`docker-compose.yml`**: Used for local development to spin up the backend along with necessary services (like a local Postgres database).

---

## 🦀 Rust Backend Workspace (Crates)

The backend is built as a highly modular Cargo workspace. Each folder is a separate crate responsible for a specific domain:

### 🚀 Core
- **`server/`**: The main entry point binary (`helios-server`). It uses the `axum` web framework to bind the port, initialize the PostgreSQL database connection, load the initial document state, and route traffic (including serving the static frontend files and accepting WebSocket upgrades).
- **`network/`**: Manages the real-time WebSocket connections. It contains the `AppState` and `DocumentRoom` logic, handling client joins, broadcasting operations, and heartbeats to track presence.

### 🧠 Data Structures & Algorithms
- **`crdt/`**: Contains the Conflict-free Replicated Data Type (CRDT) implementation. This defines the `Document`, `Op` (Operations), `OpId`, and `OpLog` structures that allow for distributed, conflict-free text editing.
- **`ot-reconciler/`**: Implements Operational Transformation (OT) logic to assist the CRDT in resolving complex edge cases during high-concurrency edits.
- **`sync/`**: Defines the shared wire-protocol payloads (`ClientMessage`, `ServerMessage`) used for communication between the frontend WebSocket client and the backend server.
- **`presence/`**: Manages cursor positions, selections, and live user tracking (names, colors, last seen).

### ⚙️ Utility Crates
- **`storage/`**: Interfaces for database operations (e.g., saving and loading operations from PostgreSQL).
- **`auth/`**: Infrastructure for user authentication and authorization.
- **`telemetry/`**: Logging, metrics, and tracing instrumentation.
- **`wasm-runtime/`**: Extensibility features allowing for WebAssembly execution.
- **`proto/`**: Protocol Buffers / gRPC definitions for potential microservice communication.
- **`bench/`**: Benchmarking tools to test the performance of the CRDT and network layers.

---

## 🎨 Frontend Application

The frontend is a modern React application built with TypeScript and Vite. It is completely decoupled from the backend but designed to connect instantly via WebSockets.

**`frontend/`**
- **`package.json`**: NPM dependencies and scripts (e.g., `npm run dev` uses `concurrently` to boot both Vite and the Rust server).
- **`src/`**
  - **`App.tsx`**: The root component. Handles the UI layout, conditionally rendering the Login screen or the main Editor, and managing the dynamic display name and color.
  - **`lib/ws.ts`**: The core `HeliosClient` class. It manages the raw WebSocket connection, parses incoming CRDT operations, handles reconnections, and emits typed events.
  - **`hooks/useHelios.ts`**: The bridge between React and the `HeliosClient`. It sets up the 2-second heartbeat interval, manages local optimistic updates, and synchronizes the CRDT state into React's memory.
  - **`components/`**
    - **`Editor.tsx`**: The main collaborative text area. It renders the user's input, displays floating collaborator tags, and triggers text change events.
    - **`Login.tsx`**: A beautiful glassmorphism entry screen where users enter their name and the Document ID they want to join.
  - **`index.css`**: Global styles, CSS variables, and modern visual aesthetics (gradients, glassmorphism, animations).

---

## 🧪 Testing

- **`tests/`**: Contains end-to-end integration tests that boot up the `helios-server` and connect simulated WebSocket clients to verify concurrency, presence broadcasting, and data convergence across the entire stack.
