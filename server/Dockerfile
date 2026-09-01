FROM rust:1.88-slim AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
# Empty by default (cargo picks its own parallelism). Set to 1 on small hosts —
# parallel rustc processes are what blow the memory budget on a 1-core/512MB box.
ARG CARGO_BUILD_JOBS=
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /build/target/release/hookbot-server /app/hookbot-server
RUN mkdir -p /app/data/firmware /app/certs
CMD ["/app/hookbot-server"]
