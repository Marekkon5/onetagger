<p align='center'>
    <img alt='Logo' src='https://raw.githubusercontent.com/Marekkon5/onetagger/master/assets/onetagger-logo-github.png'>
</p>
<h1 align='center'>The ultimate cross-platform tagger for DJs</h1>

<h3 align='center'><b>
<a href='https://onetagger.github.io/'>Website</a> | <a href='https://github.com/Marekkon5/onetagger/releases/'>Latest Release</a>
</b></h3>
<br>

<p align='center'>
    <img alt='Version Badge' src='https://img.shields.io/github/v/release/marekkon5/onetagger?label=Latest%20Release'>
    <img alt='Supported OS' src='https://img.shields.io/badge/OS-Windows%2C%20Mac%20OS%2C%20Linux-orange'>
    <img alt='Build Status' src='https://img.shields.io/github/actions/workflow/status/marekkon5/onetagger/build.yml?branch=master'>
</p>

<h3 align='center'><b></b></h3>
<hr>

Cross-platform music tagger.
It can fetch metadata from Beatport, Traxsource, Juno Download, Discogs, Musicbrainz and Spotify.
It is also able to fetch Spotify's Audio Features based on ISRC & exact match. 
There is a manual tag editor and quick tag editor which lets you use keyboard shortcuts. Written in Rust, Vue.js and Quasar.

MP3, AIFF, FLAC, M4A (AAC, ALAC) supported.

*For more info and tutorials check out our [website](https://onetagger.github.io/).*

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4


## Installing

You can download latest binaries from [releases](https://github.com/Marekkon5/onetagger/releases)


## Docker

OneTagger can also run as a headless web service in a container. The image builds `onetagger-cli` only (no GUI / webkit / gtk dependencies) and serves the embedded web UI over plain HTTP on port `36913`. The Vue client picks `ws://` or `wss://` automatically from the page's protocol, so the same image works behind both HTTP and HTTPS reverse proxies.

### Build the image

From a checkout of this repository:

```
docker compose build
```

Or without compose:

```
docker build -t onetagger-local .
```

Multi-stage build: `rust:1-bookworm` compiles the Vue client (pnpm) and `onetagger-cli`, then `debian:bookworm-slim` is the runtime with `libasound2`, `libssl3`, and `ca-certificates`. Final image is around 210 MB.

### Run with docker compose

A `docker-compose.yml` is provided at the repo root. Adjust the volume paths to your music library, then:

```
mkdir -p data/config && sudo chown -R 1000:1000 data/config
docker compose up -d
docker compose logs -f
```

### Run with docker run

Equivalent invocation without compose:

```
docker run -d \
  --name onetagger \
  --restart unless-stopped \
  -p 127.0.0.1:36913:36913 \
  -v "$(pwd)/data/config:/home/onetagger/.config/onetagger" \
  -v /path/to/your/music:/music \
  --user 1000:1000 \
  --read-only \
  --tmpfs /tmp \
  --cap-drop ALL \
  --security-opt no-new-privileges:true \
  onetagger-local:latest
```

### Options reference

#### Port

The server listens on TCP `36913` inside the container. The port is a compile-time constant (`onetagger-shared::PORT`) — to expose it on a different host port just remap, e.g. `-p 127.0.0.1:8080:36913`.

#### Volumes

| Container path | Purpose |
|---|---|
| `/home/onetagger/.config/onetagger` | OneTagger config: Spotify/Discogs OAuth tokens, custom platform settings, autotagger profiles. Persist this across rebuilds. |
| Any path you mount your music to | Music library. Mount read-write where tag writes are allowed; mount read-only (append `:ro`) for browse-only paths. A common pattern is a writable "staging" path and a read-only "main" path. |

#### User

The image creates a `onetagger` user at UID 1000, GID 1000. Run as a different host UID with `--user $(id -u):$(id -g)` so tag writes keep correct ownership on the mounted library. The numeric UID doesn't need to exist in `/etc/passwd` inside the container.

#### CLI arguments

The default command is `server --expose`. Override via compose `command:` or by appending args to `docker run`:

| Flag | Effect |
|---|---|
| `--expose`, `-e` | **Required in the container** — binds the server to `0.0.0.0:36913`. Without it the bind is `127.0.0.1` inside the container, which the host can't reach. |
| `--path <path>`, `-p <path>` | Initial path shown in the UI's folder picker. e.g. `--path /music`. |
| `--browser`, `-b` | Not useful headless — would try to open a browser inside the container. |

Example: set the initial UI path via compose

```yaml
services:
  onetagger:
    image: onetagger-local:latest
    command: ["server", "--expose", "--path", "/music"]
```

The other `onetagger-cli` subcommands (`autotagger`, `audiofeatures`, `renamer`, `authorize-spotify`) can be invoked directly against the same image:

```
docker run --rm -v /path/to/music:/music onetagger-local:latest autotagger --path /music
```

#### Recommended hardening

The provided `docker-compose.yml` enables these by default:

| Flag | Effect |
|---|---|
| `--read-only` | Root filesystem mounted read-only. All persistent state lives in the explicit volumes. |
| `--tmpfs /tmp` | Writable scratch space; needed because the root FS is read-only. |
| `--cap-drop ALL` | Drops all Linux capabilities — OneTagger needs none. |
| `--security-opt no-new-privileges:true` | Standard hardening. |

#### Behind a reverse proxy

The container speaks plain HTTP. To put it behind HTTPS, terminate TLS at a reverse proxy (Caddy, Nginx, Traefik, etc.) and forward to `127.0.0.1:36913`. WebSocket upgrades on `/ws` must be passed through; the UI's WebSocket scheme is selected automatically from the page's protocol.

Example Nginx fragment:

```
location / {
    proxy_pass http://127.0.0.1:36913;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```


## Credits
Bas Curtiz - UI, Idea, Help  
SongRec (Shazam support) - https://github.com/marin-m/SongRec

## Support
You can support this project by donating on [PayPal](https://paypal.me/marekkon5) or [Patreon](https://www.patreon.com/onetagger)

## Compilling

### Linux & Mac
Install dependencies: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/package-manager/), [pnpm](https://pnpm.io/installation)

**Install remaining dependencies**
```
sudo apt install -y lld autogen libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.1-dev
```

**Compile UI**
```
cd client
pnpm i
pnpm run build
cd ..
```

**Compile**
```
cargo build --release
```
Output will be in: `target/release/onetagger`


### Windows
You need to install dependencies: [rustup](https://rustup.rs), [nodejs](https://nodejs.org/en/download/), [Visual Studio 2019 Build Tools](https://aka.ms/vs/16/release/vs_buildtools.exe), [pnpm](https://pnpm.io/installation)

**Compile UI:**
```
cd client
pnpm i
pnpm run build
cd ..
```

**Compile OneTagger:**
```
cargo build --release
```

Output will be inside `target\release` folder.
