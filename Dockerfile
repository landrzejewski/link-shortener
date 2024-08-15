FROM rust:1.80 as builder
WORKDIR /usr/src/link-shortener
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/link-shortener /usr/local/bin/link-shortener
CMD ["link-shortener"]
