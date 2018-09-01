# graceful-shutdown

> For the times where you want to terminate things. Humanely.

This command reads a list of processes from STDIN and shuts them all down
gracefully. Commands will be matched using regular expressions.

Input supports comments using "#", making it simple to have saved recipes.

```bash
cat ~/.config/graceful-shutdown/browsers
```

```
# Shuts down all browsers

qutebrowser

firefox # Vanilla Firefox
firefox-dev.* # Firefox Developer Edition

(google-)?chrom(e|ium) # Google Chrome + Chromium
```

```bash
graceful-shutdown --mine < ~/.config/graceful-shutdown/browsers
```

## Options

By default all matching processes will receive `SIGTERM`, then the command will
wait up to 5 seconds for all processes to terminate and then send `SIGKILL` to
them.

The signals and wait time can be set through the command-line, and the wait
time can also be disabled, as well as the final kill strike.

```bash
# Wait up to 15 seconds for Firefox
echo "firefox" | graceful-shutdown --wait-time 15

# Don't even wait, just forcefully kill all open man pages immediately
echo "^man$" | graceful-shutdown --wait-time 0

# Wait, but give up after the timeout instead of killing the process
if ! echo "^[nmg]?vim" | graceful-shutdown --quiet --wait-time 30 --no-kill; then
  echo "Failed to exit all instances of vim in 30 seconds…"
fi
```

### Signals

To list supported signals you can invoke the command with `--list-signals`.


### Matching on whole command

Sometimes you want to match processes that have been started with a specific
commandline (or to ignore those that do). For this you may use the
`--whole-command` option.

```bash
# Shut down electron shells for YakYak
# Example commandline: "/usr/lib/electron/electron /usr/share/yakyak/app"
echo "/electron .*yakyak/app$" | graceful-shutdown --whole-command --mine

# Only shut down the main Spotify process, not the zygote and renderer child
# processes.
# Example commandline:
#   "/usr/share/spotify/spotify --force-device-scale-factor=1"
#   "/usr/share/spotify/spotify"
# Should not match:
#   "/usr/share/spotify/spotify --type=zygote --no-sandbox --fo…"
#   "/usr/share/spotify/spotify --type=renderer --force-device-…"
echo "/spotify( --force-device|$)" | graceful-shutdown --whole-command --mine
```

## Platform support

Currently this software is only supported on Linux. It is possible to add more
platforms if someone would care to add support for them; PRs are welcome.

## Completions

This command comes with support for shell autocompletions for **bash**,
**zsh**, and **fish**.

You can generate and install these completions globally:

```bash
graceful-shutdown --generate-completions zsh > _graceful-shutdown
graceful-shutdown --generate-completions bash > graceful-shutdown.bash
graceful-shutdown --generate-completions fish > graceful-shutdown.fish

sudo install -Dm644 _graceful-shutdown \
  /usr/share/zsh/site-functions/_graceful-shutdown

sudo install -Dm644 graceful-shutdown.bash \
  /usr/share/bash-completion/completions/graceful-shutdown

sudo install -Dm644 graceful-shutdown.fish \
  /usr/share/fish/completions/graceful-shutdown.fish
```

If you have a local source for completions, redirect the output of the
`--generate-completions` command to the appropriate location.

## Copyright

Copyright 2018 Magnus Bergmark <magnus.bergmark@gmail.com>

Code is released under MIT license, see `LICENSE`.
