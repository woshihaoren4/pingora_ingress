FROM rust:1.75.0-alpine3.19 as build
WORKDIR /pingora-ingress
#COPY build.toml .cargo/config.toml
#COPY vendor vendor
#COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --release --target aarch64-unknown-linux-musl


#FROM frolvlad/alpine-glibc:alpine-3_glibc-2.34
FROM alpine:3.19
#ENV RUST_LOG warn
EXPOSE 80 443
WORKDIR /root/
COPY --from=build /pingora-ingress/target/release/pingora-ingress .

CMD ["./pingora-ingress", "run"]