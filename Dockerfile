ARG RUST_VERS=1.43.0
ARG ALPINE_VERS=3.11

FROM rust:${RUST_VERS}-alpine${ALPINE_VERS} as builder

# https://github.com/rust-lang/rust/issues/59302
ENV RUSTFLAGS="-C target-feature=-crt-static"

# Add tools we'll use to lint.
RUN apk add \
    build-base \
    openssl-dev
RUN rustup component add clippy
RUN rustup component add rustfmt

# Cache compiled dependencies.
RUN USER=root cargo new project
WORKDIR /project
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release
RUN cargo build
RUN cargo clippy --all-targets --all-features

# Build our code.
COPY src/ /project/src/
RUN cargo fmt --all -- --check
RUN cargo clippy --all-targets --all-features
RUN cargo test
RUN cargo build --release

FROM alpine:${ALPINE_VERS} as target
COPY --from=builder /project/target/release/daemon /bin/
