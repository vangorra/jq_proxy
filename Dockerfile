FROM alpine:3.13.6

RUN apk add --no-cache --virtual .build-deps \
    curl-dev \
    jq-dev \
    oniguruma-dev \
    cargo \
    rust

RUN apk add --no-cache \
    curl \
    jq \
    oniguruma

COPY . /build

RUN cd /build \
    && JQ_LIB_DIR=/usr/lib cargo build --release --target-dir /app \
    && rm -rf /build

RUN apk del .build-deps

ENTRYPOINT ["/app/release/jq_proxy"]
