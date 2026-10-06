#!/usr/bin/env python3
"""Privately configure the provider credentials used by the Mixsets worker."""

from __future__ import annotations

import argparse
import base64
import getpass
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.parse import urlencode
from urllib.request import Request, urlopen


ROOT = Path(__file__).resolve().parents[2]
ENV_FILE = Path(__file__).with_name('.env')
COMPOSE_FILE = Path(__file__).with_name('compose.yml')
HEALTH_URL = 'http://127.0.0.1:8793/api/health'
SAFE_VALUE = re.compile(r"^[^\s#'\"$`\\]+$")


def safe_value(value: str) -> str:
    if not value or not SAFE_VALUE.fullmatch(value):
        raise ValueError('Credential is empty or contains characters unsupported by the VPS .env file')
    return value


def replace_values(text: str, values: dict[str, str]) -> str:
    """Preserve unrelated settings and comments when replacing credential keys."""
    lines: list[str] = []
    seen: set[str] = set()
    for line in text.splitlines():
        key = line.split('=', 1)[0]
        if key in values:
            if key not in seen:
                lines.append(f'{key}={values[key]}')
                seen.add(key)
        else:
            lines.append(line)
    lines.extend(f'{key}={value}' for key, value in values.items() if key not in seen)
    return '\n'.join(lines) + '\n'


def request_json(url: str, headers: dict[str, str], data: bytes | None = None) -> dict:
    request = Request(url, data=data, headers=headers, method='POST' if data else 'GET')
    try:
        with urlopen(request, timeout=15) as response:
            return json.load(response)
    except HTTPError as error:
        reason = 'rate limited the check' if error.code == 429 else 'rejected the credentials' if error.code in (401, 403) else 'returned an error'
        raise RuntimeError(f'Provider {reason} (HTTP {error.code})') from None
    except URLError:
        raise RuntimeError('Provider could not be reached from this server') from None


def validate_soundcloud(client_id: str, client_secret: str) -> None:
    basic = base64.b64encode(f'{client_id}:{client_secret}'.encode()).decode()
    token = request_json(
        'https://secure.soundcloud.com/oauth/token',
        {
            'Accept': 'application/json',
            'Content-Type': 'application/x-www-form-urlencoded',
            'Authorization': f'Basic {basic}',
        },
        urlencode({'grant_type': 'client_credentials'}).encode(),
    ).get('access_token')
    if not token:
        raise RuntimeError('SoundCloud did not return an access token')
    request_json(
        'https://api.soundcloud.com/tracks?q=freetekno&limit=1&linked_partitioning=true',
        {'Accept': 'application/json', 'Authorization': f'OAuth {token}'},
    )


def validate_discogs(token: str) -> None:
    request_json(
        'https://api.discogs.com/database/search?q=spiral+tribe&type=artist&per_page=1',
        {
            'Accept': 'application/vnd.discogs.v2.discogs+json',
            'User-Agent': 'SYCO23-Mixsets/0.1 (+metadata-enrichment)',
            'Authorization': f'Discogs token={token}',
        },
    )


def validate_youtube(api_key: str) -> None:
    request_json(
        'https://www.googleapis.com/youtube/v3/videos?' + urlencode({
            'part': 'id', 'id': 'dQw4w9WgXcQ', 'key': api_key,
        }),
        {'Accept': 'application/json'},
    )


def save_env(env_file: Path, values: dict[str, str]) -> None:
    existing = env_file.read_text() if env_file.exists() else env_file.with_name('.env.example').read_text()
    updated = replace_values(existing, values)
    temp_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile('w', encoding='utf-8', dir=env_file.parent, delete=False) as temporary:
            temp_path = Path(temporary.name)
            os.chmod(temp_path, 0o600)
            temporary.write(updated)
        os.replace(temp_path, env_file)
    finally:
        if temp_path and temp_path.exists():
            temp_path.unlink()


def provider_state(provider: str) -> tuple[str, str]:
    payload = request_json(HEALTH_URL, {})
    entry = next((item for item in payload.get('providers', []) if item.get('id') == provider), None)
    if not entry:
        raise RuntimeError(f'{provider} was absent from the worker health response')
    return entry['state'], entry['detail']


def restart_and_check(provider: str, env_file: Path) -> None:
    subprocess.run(
        ['docker', 'compose', '--env-file', str(env_file), '-f', str(COMPOSE_FILE), 'up', '-d', '--no-deps', '--force-recreate', 'api'],
        cwd=ROOT,
        check=True,
    )
    for _ in range(20):
        try:
            state, detail = provider_state(provider)
            if state == 'ready':
                print(f'{provider}: ready — {detail}')
                return
        except (RuntimeError, ValueError, OSError):
            pass
        time.sleep(2)
    state, detail = provider_state(provider)
    raise RuntimeError(f'Credentials were saved, but the worker reports {provider}: {state} — {detail}')


def main() -> int:
    parser = argparse.ArgumentParser(description='Set up Mixsets provider credentials on VPS-L without exposing them in the browser or shell history.')
    parser.add_argument('provider', choices=['soundcloud', 'discogs', 'youtube'])
    parser.add_argument('--check', action='store_true', help='Check current worker state without changing credentials')
    args = parser.parse_args()

    try:
        if args.check:
            state, detail = provider_state(args.provider)
            print(f'{args.provider}: {state} — {detail}')
            return 0 if state == 'ready' else 1
        if not sys.stdin.isatty() or not sys.stderr.isatty():
            raise RuntimeError('Run interactively in a private VPS terminal so credentials are not echoed or stored in shell history')
        if args.provider == 'soundcloud':
            client_id = safe_value(getpass.getpass('SoundCloud Client ID: '))
            client_secret = safe_value(getpass.getpass('SoundCloud Client Secret: '))
            validate_soundcloud(client_id, client_secret)
            values = {'SOUNDCLOUD_CLIENT_ID': client_id, 'SOUNDCLOUD_CLIENT_SECRET': client_secret}
        elif args.provider == 'discogs':
            token = safe_value(getpass.getpass('Discogs personal API token: '))
            validate_discogs(token)
            values = {'DISCOGS_TOKEN': token}
        else:
            api_key = safe_value(getpass.getpass('YouTube Data API key: '))
            validate_youtube(api_key)
            values = {'YOUTUBE_API_KEY': api_key}
        print(f'{args.provider}: credentials verified with the provider')
        save_env(ENV_FILE, values)
        print('Credentials saved privately; restarting the Mixsets worker…')
        restart_and_check(args.provider, ENV_FILE)
        return 0
    except (RuntimeError, ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f'Setup failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
