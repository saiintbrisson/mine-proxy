# Default build image to rust-musl-builder
ARG BASE_IMAGE=ekidd/rust-musl-builder:latest
FROM ${BASE_IMAGE} AS builder

ADD --chown=rust:rust . ./
RUN cargo build --release

FROM scratch
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/mine-proxy \
    /mine-proxy
CMD ["/mine-proxy"]
