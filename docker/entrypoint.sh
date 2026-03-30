#!/bin/sh
set -eu

# Drop privileges for bind mounts so tagged files are not written as root.
uid="${PUID:-1000}"
gid="${PGID:-1000}"

if [ "$(id -u)" = "0" ]; then
    exec gosu "${uid}:${gid}" /usr/local/bin/onetagger-cli "$@"
fi

exec /usr/local/bin/onetagger-cli "$@"

