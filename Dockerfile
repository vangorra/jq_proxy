FROM rust:1.55.0-bullseye as build
WORKDIR /build
ENV JQ_LIB_DIR=/usr/lib/x86_64-linux-gnu

RUN apt-get update \
    && apt-get install --assume-yes \
        libjq1 \
        libjq-dev \
        libonig-dev \
        libonig5

# Download rust dependencies.
COPY Cargo.lock Cargo.toml ./
RUN mkdir .cargo \
    && cargo vendor > .cargo/config.toml

# Build statically linked binary.
COPY . .
RUN cargo install --path .

# Build minimal image from compiled binary.
FROM debian:bullseye-slim
RUN apt-get update \
    && apt-get install --assume-yes \
        libjq1 \
    && apt-get clean \
    && rm -rf /var/cache/apt \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/local/cargo/bin/jq_proxy /usr/local/bin
ENTRYPOINT ["jq_proxy"]
