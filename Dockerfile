FROM rust:latest AS builder

# Install musl-tools and other necessary dependencies
RUN apt-get update && apt-get install -y musl-tools && rustup target add x86_64-unknown-linux-musl

WORKDIR /app
COPY . .

RUN cargo build --release --target x86_64-unknown-linux-musl

FROM alpine:latest
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/ec2_terminator ./bootstrap

# Ensure the binary is executable
RUN chmod +x ./bootstrap

# AWS Lambda requires the handler to be called "bootstrap"
ENTRYPOINT ["./bootstrap"]

