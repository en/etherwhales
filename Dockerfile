FROM rust:1.43 as builder
WORKDIR /usr/src/etherwhales
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
# RUN apt-get update && apt-get install -y extra-runtime-dependencies
COPY --from=builder /usr/local/cargo/bin/etherwhales /usr/local/bin/etherwhales
CMD ["etherwhales"]
