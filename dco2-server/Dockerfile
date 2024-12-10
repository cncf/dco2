# Build dco2-server
FROM rust:1-alpine3.21 AS builder
RUN apk --no-cache add musl-dev perl make
WORKDIR /dco2
COPY Cargo.* ./
COPY dco2 dco2
COPY dco2-aws-lambda/Cargo.* dco2-aws-lambda
COPY dco2-server dco2-server
WORKDIR /dco2/dco2-server
RUN cargo build --release

# Final stage
FROM alpine:3.21.0
RUN apk --no-cache add ca-certificates && addgroup -S dco2 && adduser -S dco2 -G dco2
USER dco2
WORKDIR /home/dco2
COPY --from=builder /dco2/target/release/dco2-server /usr/local/bin
