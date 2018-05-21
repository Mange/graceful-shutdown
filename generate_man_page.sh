#!/usr/bin/bash

if ! hash help2man 2>/dev/null; then
  echo "You need to install help2man to generate the manpage!" > /dev/stderr
  exit 1
fi

root="$(cd "$(dirname "$0")" && pwd)"

cd "$root" && \
  cargo build && \
  help2man --no-info target/debug/graceful-shutdown > man/graceful-shutdown.1
