# Build stage
FROM rust:1.86-slim as builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates
COPY %Rules ./%Rules
COPY prompts ./prompts
COPY anansi.toml.example ./anansi.toml.example

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/anansi2 /usr/local/bin/anansi2

ENV ANANSI_ROOT=/vault

VOLUME ["/vault"]

EXPOSE 3738

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD curl -f http://localhost:3738/health || exit 1

ENTRYPOINT ["anansi2"]
CMD ["serve", "--root", "/vault"]
