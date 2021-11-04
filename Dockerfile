FROM rust:1.56-slim-buster as builder

WORKDIR /usr/src

COPY . .

RUN cargo build --release

FROM debian:buster-slim

RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install wget -y

COPY --from=builder /usr/src/target/release/teeny .
COPY --from=builder /usr/src/target/release/import .

CMD ["./teeny"]