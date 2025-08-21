FROM rust:1.79 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p agentstate-server

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /app/target/release/agentstate-server /agentstate-server
EXPOSE 8080
ENTRYPOINT ["/agentstate-server"]
