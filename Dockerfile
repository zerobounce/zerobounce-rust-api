# ZeroBounce Rust SDK – test image (the MSRV, 1.88; reqwest 0.13 and current crates.io
# transitives need edition2024 / rustc newer than 1.83)
# The base is pinned by digest; Dependabot moves the digest and keeps 1.88.
FROM rust:1.88-bookworm@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0

WORKDIR /app

COPY . .

# Cargo.lock is not tracked, so resolve to crate versions that support the image's Rust
# (1.88), as the MSRV CI job does.
ENV CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback

# Download the crates at build time, so the tests run without network access. Some crates
# unpack files readable only by their owner (root), so open the cache to the test user.
RUN cargo fetch && chmod -R a+rX /usr/local/cargo/registry

# The tests run as a non-root user that cannot write /app, so the build output goes to
# /tmp. The crate cache (filled above) is only read at test time.
ENV CARGO_TARGET_DIR=/tmp/target

USER 65532:65532

# Unit, integration and doc tests; the live suite needs `--features live` and is not run.
CMD ["cargo", "test", "--offline", "--no-fail-fast"]
