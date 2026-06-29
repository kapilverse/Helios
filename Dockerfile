FROM node:22-slim as frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

FROM rust:1.78-slim as backend
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crdt/ crdt/
COPY ot-reconciler/ ot-reconciler/
COPY network/ network/
COPY sync/ sync/
COPY presence/ presence/
COPY storage/ storage/
COPY auth/ auth/
COPY telemetry/ telemetry/
COPY bench/ bench/
COPY server/ server/
COPY proto/ proto/
RUN cargo build --release --bin helios-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash helios
USER helios
WORKDIR /home/helios
COPY --from=backend /app/target/release/helios-server .
COPY --from=frontend /app/dist static/
EXPOSE 3000
ENV RUST_LOG=info
CMD ["./helios-server"]
