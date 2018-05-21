# graceful-shutdown

> For the times where you want to terminate things. Humanely.

This command reads a list of processes from STDIN and shuts them all down
gracefully. Commands will be matched using regular expressions.

Input supports comments using "#", making it simple to have saved recipes.

```bash
cat ~/.config/graceful-shutdown/browsers
# Shuts down all browsers

qutebrowser

firefox # Vanilla Firefox
firefox-dev.* # Firefox Developer Edition

(google-)?chrom(e|ium) # Google Chrome + Chromium
$ graceful-shutdown --mine < ~/.config/graceful-shutdown/browsers
```

## Options

By default all matching processes will receive `SIGTERM`, then the command will
wait up to 5 seconds for all processes to terminate and then send `SIGKILL` to
them.

The signals and wait time can be set through the command-line, and the wait
time can also be disabled, as well as the final kill strike.

### Signals

To list supported signals you can invoke the command with `--list-signals`.

## Copyright

Copyright 2018 Magnus Bergmark <magnus.bergmark@gmail.com>

Code is released under MIT license, see `LICENSE`.
