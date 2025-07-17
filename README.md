# Birdwatcher 3.0.0

[![Build and Test](https://github.com/alice-lg/lightwatcher/actions/workflows/rust.yml/badge.svg)](https://github.com/alice-lg/lightwatcher/actions/workflows/rust.yml)

This is an experimental lightweight clone of the birdwatcher.
It is written in rust and focusses on a small memory footprint
and a minimal feature set in order to work with Alice.

**NOTICE:** This software is work in progress and should be
considered a 'preview' release.

## Configuration

The is configured entirely using the following environment variables:

`LIGHTWATCHER_LISTEN` (default: `127.0.0.1:8181`)

`LIGHTWATCHER_BIRDC`  (default: `/var/run/bird/bird.ctl`)

`LIGHTWATCHER_ROUTES_CACHE_MAX_ENTRIES` (default: `25`)

`LIGHTWATCHER_ROUTES_CACHE_TTL` (default: `300`)

`LIGHTWATCHER_NEIGHBORS_CACHE_MAX_ENTRIES` (default: `1`)

`LIGHTWATCHER_NEIGHBORS_CACHE_TTL` (default: `300`)

`LIGHTWATCHER_ROUTES_WORKER_POOL_SIZE` (default: `<cpu cores>`)

`LIGHTWATCHER_RATE_LIMIT_REQUESTS` (default: `512`)

`LIGHTWATCHER_RATE_LIMIT_WINDOW`  (default: `60` seconds)


## Contributing

Please feel free to test this software and create issues.
An issue should contain the request and idealy a dump of
the birdc result.


