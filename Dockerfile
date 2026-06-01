# ---- Build Stage ----
FROM rust:1.83-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
# Cache dependencies
RUN mkdir src && echo 'fn main(){}' > src/main.rs && cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY . .
RUN touch src/main.rs && cargo build --release

# ---- Runtime Stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash litepay
USER litepay
WORKDIR /home/litepay

COPY --from=builder /app/target/release/litepay-server /usr/local/bin/litepay-server
COPY litepay.example.toml ./litepay.toml

ENV RUST_LOG=litepay=info
EXPOSE 9000

CMD ["litepay-server"]
