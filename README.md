# jq_proxy
A simple http service which proxies json through the popular json tool jq.

## Quickstart
```bash
$ docker run --rm --publish 8080:8080 --volume $(pwd)/config.yaml:/config.yaml vangorra/jq_proxy --config-file-path /config.yaml
$ curl http://localhost:8080/proxy
```

## Building
```bash
# With docker
$ docker build --tag jq_proxy .
$ docker run --rm --publish 8080:8080 --volume $(pwd)/config.yaml:/config.yaml jq_proxy --config-file-path /config.yaml

# With rust
$ cargo run -- --config ./config.yaml
```
