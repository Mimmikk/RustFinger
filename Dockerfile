FROM rust:1.82-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm src/main.rs
COPY src ./src
RUN touch src/main.rs && cargo build --release
FROM alpine:latest
RUN apk add --no-cache curl
COPY --from=builder /app/target/release/rustfinger /rustfinger
COPY urns.yml /urns.yml
RUN mkdir /config
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/healthz || exit 1

# Entrypoint
ENTRYPOINT ["/rustfinger"] 