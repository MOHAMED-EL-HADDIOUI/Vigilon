# syntax=docker/dockerfile:1
# Pinned majors: float at your own risk (supply chain + reproducibility).
FROM node:22.17-alpine AS frontend
WORKDIR /web
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.89-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY core core
COPY platform platform
COPY agent agent
COPY api api
RUN cargo build -p vigilon-api --release --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/vigilon /usr/local/bin/vigilon
COPY --from=frontend /web/dist /usr/share/vigilon/frontend
# NOTE: 0.0.0.0 exposes the API (incl. fleet proxy) to the network with no
# authentication. Only publish this port to trusted networks, or front it
# with a reverse proxy that enforces auth. The daemon logs a warning on
# every non-loopback bind.
ENV VIGILON_BIND=0.0.0.0:8745
ENV VIGILON_FRONTEND=/usr/share/vigilon/frontend
ENV VIGILON_DATA=/data
EXPOSE 8745
VOLUME ["/data"]
ENTRYPOINT ["vigilon"]
