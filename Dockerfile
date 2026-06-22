# Build stage
FROM rust:latest AS builder

RUN apt-get update && apt-get install -y pkg-config musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY build.rs ./build.rs
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY %Rules ./%Rules
COPY anansi.toml.example ./anansi.toml.example

# Git SHA passed as build arg from CI; build.rs reads GIT_SHA env if git isn't available
ARG GIT_SHA=unknown
ENV GIT_SHA=${GIT_SHA}

RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/anansi2 /usr/local/bin/anansi2
# Bake in the skill files so the inbox pipeline works out of the box.
# Skills live at /app/skills — override via skills_dir in anansi.toml if needed.
COPY skills/ /app/skills/

EXPOSE 3738

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD curl -f http://localhost:3738/health || exit 1

CMD ["anansi2", "serve", "--root", "/data"]
