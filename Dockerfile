# syntax=docker/dockerfile:1.4

FROM rust:1.69 AS builder
WORKDIR /root
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    cargo install cargo-strip
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/root/target \
    cargo build --release && \
    cargo strip && \
    mv /root/target/release/arso_exporter /root


FROM debian:bullseye-slim
RUN apt-get update && \
    apt-get upgrade -y && \
    DEBIAN_FRONTEND=noninteractive \
      apt-get install -y --no-install-recommends \
        ca-certificates \
    && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m --uid=1000 arso
COPY --from=builder /root/arso_exporter /arso_exporter
COPY entrypoint.sh /entrypoint.sh

USER arso
ENTRYPOINT [ "/entrypoint.sh" ]
