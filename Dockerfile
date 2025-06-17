FROM docker.io/library/rust:alpine as builder
ENV SYSROOT=/dummy SQLITE3_STATIC=1 SQLITE_LIB_DIR=/usr/lib/ LIBPQ_STATIC=1

WORKDIR /wd
COPY . /wd
RUN cargo fetch 
RUN apk add --no-cache musl-dev sqlite-static openssl-dev openssl-libs-static pkgconf git libpq-dev
RUN cargo build --bins --release

FROM alpine:latest
ARG version=unknown
ARG release=unreleased

ADD https://homelab.lan/root.crt /usr/local/share/ca-certificates/
RUN apk add --no-cache ca-certificates && update-ca-certificates

COPY --from=builder /wd/target/release/bv /
CMD ["/bv"]
