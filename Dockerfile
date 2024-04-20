#FROM rust:1.75 as build
#WORKDIR /pingora-ingress
##COPY build.toml .cargo/config.toml
##COPY vendor vendor
##COPY Cargo.toml Cargo.lock ./
#COPY . .
#RUN cargo build --release


FROM frolvlad/alpine-glibc:latest
#FROM alpine:3.16
#ENV RUST_LOG warn
EXPOSE 80 443
WORKDIR /root/
#COPY --from=build /pingora-ingress/target/release/pingora-ingress .
COPY target/x86_64-unknown-linux-musl/release/pingora-ingress .

CMD ["./pingora-ingress", "run"]