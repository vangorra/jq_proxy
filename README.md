# jq_proxy
A simple http service which proxies json through the popular json tool jq.

## Quickstart
```bash
$ docker run --rm --publish 8080:8080 vangorra/jq_proxy
$ curl http://localhost:8080/url=<URL of target>&query=<jq query to run>
```

## Building
```bash
# With docker
$ docker build --name jq_proxy .

# With rust
$ cargo build
```
