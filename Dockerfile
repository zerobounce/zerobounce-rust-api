# ZeroBounce Rust SDK – test image (Rust 1.83+ for Cargo.lock v4)
FROM rust:1.83-bookworm

WORKDIR /app

COPY . .

# Full test suite (unit + integration; network allowed in container)
CMD ["cargo", "test", "--no-fail-fast"]
