FROM rust:1.78-slim as builder

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

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash helios
USER helios
WORKDIR /home/helios

COPY --from=builder /app/target/release/helios-server .
COPY server/static/ static/

EXPOSE 3000

ENV RUST_LOG=info
CMD ["./helios-server"]
