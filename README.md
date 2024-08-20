# Lightwatcher

This is an experimental lightweight clone of the birdwatcher.
It is written in rust and focusses on a small memory footprint
and a minimal feature set in order to work with Alice.

For now, only `single table` setups are supported.

**NOTICE:** This software is work in progress and should be
considered a 'preview' release.

## Configuration

There are currently two environment variables to configure:

`LIGHTWATCHER_LISTEN` (default: `127.0.0.1:8181`)

`LIGHTWATCHER_BIRDC`  (default: `/var/run/bird/bird.ctl`)

## Contributing

Please feel free to test this software and create issues.
An issue should contain the request and idealy a dump of
the birdc result.


