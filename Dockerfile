FROM rust:latest AS builder
COPY ./Cargo.* /app/
COPY ./src /app/src
WORKDIR /app
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/fixred /usr/local/bin/fixred
RUN apt-get update && \
    apt-get install -y --no-install-recommends libcurl4 && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["/usr/local/bin/fixred"]
