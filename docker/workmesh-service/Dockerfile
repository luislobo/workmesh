FROM rust:1.85-bookworm AS builder
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p workmesh-service

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --home-dir /home/workmesh --shell /usr/sbin/nologin workmesh

WORKDIR /workspace
COPY --from=builder /src/target/release/workmesh-service /usr/local/bin/workmesh-service

USER workmesh
EXPOSE 4747

ENTRYPOINT ["/usr/local/bin/workmesh-service"]
CMD ["--host", "0.0.0.0", "--port", "4747"]
