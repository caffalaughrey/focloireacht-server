FROM rust:1-bullseye AS builder

WORKDIR /app

# Pre-copy manifest to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs && cargo build --release || true

# Copy full source
COPY . .

# Touch source so Cargo sees them as newer than the dummy-build artifacts
RUN find src -name '*.rs' | xargs touch

# Build with offline SQLx (uses .sqlx cache checked-in) and strip
ENV SQLX_OFFLINE=1
RUN cargo build --release && \
    strip /app/target/release/focloireacht-server

FROM debian:12-slim AS runtime

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates tzdata libsqlite3-0 wget; \
    rm -rf /var/lib/apt/lists/*

WORKDIR /srv

# Non-root user
RUN useradd -r -u 10001 appuser

# Copy binary
COPY --from=builder /app/target/release/focloireacht-server /usr/local/bin/focloireacht-server

# Default DB mount location inside container
ENV LEX_DB_PATH=/data/lexicon.sqlite \
    TERM_DB_PATH=/data/terminology.sqlite \
    BIND_ADDR=0.0.0.0:5005 \
    SQLX_OFFLINE=1

EXPOSE 5005

USER appuser

HEALTHCHECK --interval=10s --timeout=2s --retries=3 CMD \
  wget -qO- http://127.0.0.1:5005/health | grep '"status":"ok"' || exit 1

ENTRYPOINT ["/usr/local/bin/focloireacht-server"]

