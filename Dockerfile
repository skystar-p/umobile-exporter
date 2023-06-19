FROM rust:1.70-slim AS builder
WORKDIR /app

COPY . .

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN cargo build --release

# ------
FROM debian:stable-slim

COPY --from=builder /app/target/release/umobile-exporter /app/

CMD ["/app/umobile-exporter"]
