FROM alpine:3.13.6

RUN apk add --no-cache --virtual .build-deps \
        curl-dev \
        jq-dev \
        oniguruma-dev \
        cargo \
        rust \
    && apk add --no-cache \
        curl \
        jq \
        oniguruma \
        libgcc \
    && mkdir -p ~/.cargo \
    && echo "[http]" >> ~/.cargo/config \
    && echo "multiplexing = false" >> ~/.cargo/config

COPY . /build

RUN cd /build \
    && JQ_LIB_DIR=/usr/lib cargo build --release \
    && mv ./target/release/jq_proxy /usr/local/bin \
    && cd / \
    && rm -rf /build \
    && rm -rf ~/.cargo \
    && apk del .build-deps

ENTRYPOINT ["jq_proxy"]
