#!/usr/bin/env python3
import base64
import getpass
import hashlib
import os
import secrets
import sys
from pathlib import Path


def encoded(value: bytes) -> str:
    return base64.urlsafe_b64encode(value).decode("ascii").rstrip("=")


def main() -> int:
    env_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).parent / ".env"
    password = getpass.getpass("Curator password (12+ characters): ")
    confirmation = getpass.getpass("Confirm curator password: ")
    if password != confirmation:
        print("Passwords do not match.", file=sys.stderr)
        return 1
    if len(password) < 12 or len(password) > 1024:
        print("Password must be between 12 and 1024 characters.", file=sys.stderr)
        return 1

    salt = secrets.token_bytes(16)
    key = hashlib.scrypt(password.encode("utf-8"), salt=salt, n=16384, r=8, p=1, dklen=64)
    password_hash = f"scrypt${encoded(salt)}${encoded(key)}"
    existing = env_path.read_text(encoding="utf-8").splitlines() if env_path.exists() else []
    output = [line for line in existing if not line.startswith("CURATOR_PASSWORD_HASH=")]
    output.append(f"CURATOR_PASSWORD_HASH={password_hash}")

    env_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = env_path.with_name(f"{env_path.name}.tmp-{os.getpid()}")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as env_file:
            env_file.write("\n".join(output) + "\n")
        os.replace(temporary, env_path)
        os.chmod(env_path, 0o600)
    finally:
        if temporary.exists():
            temporary.unlink()

    print(f"Curator hash stored privately in {env_path}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())