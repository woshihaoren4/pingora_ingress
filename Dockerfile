FROM rust:1.63.0-alpine3.16 as build
WORKDIR /pingora-ingress
#COPY build.toml .cargo/config.toml
#COPY vendor vendor
#COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --release


#FROM frolvlad/alpine-glibc:alpine-3_glibc-2.34
FROM alpine3.16
#ENV RUST_LOG warn
EXPOSE 80 443
WORKDIR /root/
COPY --from=build /pingora-ingress/target/release/pingora-ingress .

CMD ["./pingora-ingress", "run"]