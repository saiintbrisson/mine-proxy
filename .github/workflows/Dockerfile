# Add rust-musl-builder as env variable
ARG BASE_IMAGE=ekidd/rust-musl-builder:latest

# Set rust-musl-builder as first stage builder
FROM ${BASE_IMAGE} AS builder

# Add source code to image
ADD --chown=rust:rust . ./

# Build the app
RUN cargo build --release

FROM scratch
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/mine-proxy \
    /usr/local/bin/
CMD /usr/local/bin/mine-proxy
