FROM rust:1.73-slim-bullseye as builder
WORKDIR /usr/src/bitcoin_bot
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y openssl ca-certificates pkg-config && update-ca-certificates
COPY --from=builder /usr/local/cargo/bin/bitcoin-bot /usr/local/bin/bitcoin-bot
CMD ["bitcoin-bot"]