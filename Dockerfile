# ZeroBounce Rust SDK – test image (1.88+; reqwest 0.13 and current crates.io
# transitives need edition2024 / rustc newer than 1.83)
FROM rust:1.88-bookworm

WORKDIR /app

COPY . .

# Full test suite (unit + integration; network allowed in container)
CMD ["cargo", "test", "--no-fail-fast"]
