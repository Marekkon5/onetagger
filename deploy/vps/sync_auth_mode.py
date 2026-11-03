#!/usr/bin/env python3
"""Copy only AUTH_MODE from a Vercel env export into the private worker env."""
import argparse
import json
import os
from pathlib import Path
import tempfile


def read_mode(source):
    values = []
    for line in Path(source).read_text().splitlines():
        if line.startswith("AUTH_MODE="):
            value = line.partition("=")[2].strip()
            if value.startswith('"'):
                value = json.loads(value)
            elif value.startswith("'") and value.endswith("'"):
                value = value[1:-1]
            values.append(value)
    if len(values) != 1 or values[0] not in ("on", "off"):
        raise ValueError("Vercel export must contain exactly one AUTH_MODE=on or off")
    return values[0]


def sync(source, target):
    mode = read_mode(source)
    target = Path(target).resolve()
    original = target.read_text() if target.exists() else ""
    lines = original.splitlines()
    if mode == "on" and not any(line.startswith("CURATOR_PASSWORD_HASH=") and
                               line.partition("=")[2].strip() for line in lines):
        raise ValueError("Configure the private curator hash before enabling auth")
    lines = [line for line in lines if not line.startswith("AUTH_MODE=")]
    lines.append("AUTH_MODE=" + mode)
    descriptor, temporary = tempfile.mkstemp(prefix=".auth-env-", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w") as output:
            os.fchmod(output.fileno(), 0o600)
            output.write("\n".join(lines) + "\n")
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, target)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return mode


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("vercel_export")
    parser.add_argument("worker_env")
    args = parser.parse_args()
    print("Synced Vercel AUTH_MODE=" + sync(args.vercel_export, args.worker_env))
