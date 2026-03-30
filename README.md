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

### Docker (CLI)

A CLI-only image can be published to GHCR as `ghcr.io/<owner>/onetagger-cli`.
On the main repository this resolves to `ghcr.io/marekkon5/onetagger-cli:latest`.
When testing from a fork, replace `marekkon5` with your GitHub namespace or override `ONETAGGER_IMAGE`.

Export the `config.json` shown by the GUI CLI dialog, then run the container with the same command you would use locally:

```sh
docker run --rm \
  -e PUID="$(id -u)" \
  -e PGID="$(id -g)" \
  -v /path/to/onetagger-data:/data \
  -v /path/to/music:/music \
  ghcr.io/marekkon5/onetagger-cli:latest \
  autotagger --config /data/config.json --path /music
```

Put the exported `config.json` inside `/path/to/onetagger-data` on the host.
OneTagger will also store `onetagger.log`, run playlists, and any cached Spotify token data in that same bind mount under `/data/onetagger`.
If your config enables `spotify`, the first run also requires a cached Spotify token at `/data/onetagger/spotify_token_cache.json`.

The included `compose.yml` targets the same GHCR image:

```sh
ONETAGGER_DATA_DIR=/absolute/path/to/onetagger-data \
ONETAGGER_MUSIC_DIR=/absolute/path/to/music \
docker compose run --rm onetagger \
  autotagger --config /data/config.json --path /music
```

### Spotify auth on remote or SSH-only Docker hosts

Spotify authorization is a one-time setup as long as the `/data` bind mount persists.
After `spotify_token_cache.json` exists in `/data/onetagger`, normal autotagger runs do not need any interactive login.

For a remote or SSH-only Docker host, run the authorization command against the same data directory you will use for tagging:

```sh
docker run --rm -it \
  -e PUID="$(id -u)" \
  -e PGID="$(id -g)" \
  -v /path/to/onetagger-data:/data \
  ghcr.io/marekkon5/onetagger-cli:latest \
  authorize-spotify \
  --client-id "YOUR_CLIENT_ID" \
  --client-secret "YOUR_CLIENT_SECRET" \
  --prompt
```

Open the printed Spotify URL in any browser, approve the app, then copy the final redirect URL from the browser address bar and paste it back into the waiting container prompt.
The redirect page itself does not need to load successfully.

For OneTagger the Spotify app redirect URI must be set to:

```text
http://127.0.0.1:36913/spotify
```

This also works for remote Docker management over SSH because `--prompt` only needs the final redirect URL, not a browser running on the Docker host itself.
If you already have a working Spotify token from another OneTagger install, you can also copy `spotify_token_cache.json` into `/path/to/onetagger-data/onetagger/` instead of re-authorizing.

The CLI also supports auto rename through the `renamer` subcommand:

```sh
docker run --rm \
  -e PUID="$(id -u)" \
  -e PGID="$(id -g)" \
  -v /path/to/onetagger-data:/data \
  -v /path/to/music:/music \
  ghcr.io/marekkon5/onetagger-cli:latest \
  renamer --path /music --template "%artist% - %title%" --preview
```

Drop `--preview` to apply the rename, add `--output /music-renamed` to write into another directory, or `--copy` to keep the source files in place.
For example, with tags `artist = "Mosimann, Walshy Fire"` and `title = "Bad Man Sound"`, the template `%artist% - %title%` keeps the joined artist list and produces `Mosimann, Walshy Fire - Bad Man Sound`.
In the current CLI build, `%artists%` resolves to the first artist only, so use `%artist%` if you want all artists in the filename.

To run tagging and renaming in a single container invocation, override the entrypoint to a shell and chain both commands:

```sh
docker run --rm \
  --user "$(id -u):$(id -g)" \
  --entrypoint /bin/sh \
  -v /path/to/onetagger-data:/data \
  -v /path/to/music:/music \
  ghcr.io/marekkon5/onetagger-cli:latest \
  -lc '
    set -e
    onetagger-cli autotagger --config /data/config.json --path /music
    onetagger-cli renamer --path /music --template "%artist% - %title%" --no-subfolders
  '
```

Use `--user` here because overriding the entrypoint bypasses the image wrapper that normally applies `PUID` and `PGID`.


## Credits
Bas Curtiz - UI, Idea, Help  
timothe - (Docker support) - https://github.com/timothe  
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
