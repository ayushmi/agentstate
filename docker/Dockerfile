FROM rust:1.81 as builder
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cargo build --release -p agentstate-server

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/agentstate-server /agentstate-server
EXPOSE 8080
ENTRYPOINT ["/agentstate-server"]
