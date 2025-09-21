FROM rust:1.80 as build
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release -p ippan-node

FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=build /app/target/release/ippan-node /usr/local/bin/ippan-node
USER 65532:65532
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/ippan-node"]
