FROM rust:1.42.0 as builder

WORKDIR /ddh/

COPY . /ddh/
RUN cargo install --path /ddh/


FROM debian:10.3-slim

LABEL org.label-schema.vcs-url="https://github.com/darakian/ddh"

COPY --from=builder /usr/local/cargo/bin/ddh /usr/local/bin/ddh

WORKDIR /target/

CMD ["ddh"]
