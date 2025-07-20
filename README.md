# Birdwatcher 3.0.0

[![Build and Test](https://github.com/alice-lg/lightwatcher/actions/workflows/rust.yml/badge.svg)](https://github.com/alice-lg/lightwatcher/actions/workflows/rust.yml)

This is a lightweight clone of the birdwatcher.

It is written in rust and focusses on a small memory footprint
and a minimal feature set in order to work with Alice.

**NOTICE:** This software is work in progress and should be
considered a 'preview' release.

## Configuration

Lightwatcher is configured entirely using environment variables:

### Server Settings

`LIGHTWATCHER_LISTEN` (default: `127.0.0.1:8181`)
 * Address and port where the HTTP server listens.

`LIGHTWATCHER_BIRD_CTL` (default: `/var/run/bird/bird.ctl`)
 * Path to BIRD control socket.

`LIGHTWATCHER_BIRD_CONNECTION_POOL_SIZE` (default: `10`)
 * Number of concurrent BIRD connections.

### Cache Settings

`LIGHTWATCHER_ROUTES_CACHE_MAX_ENTRIES` (default: `25`)
 * Maximum number of cached route queries.

`LIGHTWATCHER_ROUTES_CACHE_TTL` (default: `300`)
 * Route cache lifetime in seconds.

`LIGHTWATCHER_NEIGHBORS_CACHE_MAX_ENTRIES` (default: `1`)
 * Maximum number of cached neighbor queries.

`LIGHTWATCHER_NEIGHBORS_CACHE_TTL` (default: `300`)
 * Neighbor cache lifetime in seconds.

### Performance Settings

`LIGHTWATCHER_ROUTES_WORKER_POOL_SIZE` (default: `<cpu cores>`)
 * Number of worker threads for route parsing.

### Rate Limiting

`LIGHTWATCHER_RATE_LIMIT_REQUESTS` (default: `512`)
 * Maximum requests allowed per window.

`LIGHTWATCHER_RATE_LIMIT_WINDOW` (default: `60`)
 * Rate limit window duration in seconds.

## BIRD Configuration

Ensure BIRD uses ISO long time format:
```
timeformat base         iso long;
timeformat log          iso long;
timeformat protocol     iso long;
timeformat route        iso long;
```

### Tagging filtered routes
If you want to make use of the filtered route reasons in [Alice-LG](https://github.com/alice-lg/alice-lg), you need
to make sure that you are using BIRD 1.6.3 or up (2.x, 3.x) as you will need Large BGP Communities
(http://largebgpcommunities.net/) support.

Also please note that BIRD 1.x is end of life!

You need to add a Large BGP Community just before you filter a route, for example:

    define yourASN = 12345
    define yourFilteredNumber = 65666
    define prefixTooLong = 1
    define pathTooLong = 2

    function importScrub() {
        ...
        if (net.len > 24) then {
            print "REJECTING: ",net.ip,"/",net.len," received from ",from,": Prefix is longer than 24: ",net.len,"!";
            bgp_large_community.add((YourASN,yourFilteredNumber,prefixTooLong));
            return false;
        }
        if (bgp_path.len > 64) then {
            print "REJECTING: ",net.ip,"/",net.len," received from ",from,": AS path length is ridiculously long: ",bgp_path.len,"!";
            bgp_large_community.add((yourASN,yourFilteredNumber,pathTooLong));
            return false;
        }
        ...
        return true;
    }

    function importFilter() {
        ...
        if !(importScrub()) then reject;
        ...
        accept;
    }


## Monitoring

An enpoint for monitoring the service available under `/health`
and contains information about the current version and the
connection to the BIRD daemon.

## Troubleshooting

**Cannot connect to BIRD socket**
Check that BIRD is running and the socket path is correct. Verify permissions:
```bash
ls -l /var/run/bird/bird.ctl
```

## Contributing

Please feel free to test this software and create issues.
An issue should contain the request and idealy a dump of
the birdc result.


