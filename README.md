<p align='center'>
    <img alt='Logo' src='assets/onetagger-logo-github.png'>
</p>
<h1 align='center'>The ultimate cross-platform tagger for DJs</h1>

<h3 align='center'><b>
<a href='#installing'>Get OneTagger</a> | <a href='https://onetagger.github.io/'>Upstream tutorials</a> | <a href='https://github.com/fantuz/onetagger/discussions'>Discussions</a>
</b></h3>
<br>

<!-- project-badges:start -->
<p align="center">
  <a href="https://github.com/fantuz/onetagger/actions/workflows/build.yml?query=branch%3Amaster"><img src="https://github.com/fantuz/onetagger/actions/workflows/build.yml/badge.svg?branch=master" alt="Tests and packages" height="20"></a>
  <a href="#installing"><img src="https://img.shields.io/badge/downloads-Actions%20artifacts-2e7d32" alt="Downloads: GitHub Actions artifacts" height="20"></a>
  <br>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/fantuz/onetagger" alt="GNU GPL v3 license" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/stargazers"><img src="https://img.shields.io/github/stars/fantuz/onetagger?style=flat" alt="GitHub stars" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/discussions"><img src="https://img.shields.io/badge/discussions-community-507269" alt="Community discussions" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/commits/master"><img src="https://img.shields.io/github/last-commit/fantuz/onetagger/master" alt="Last commit on master" height="20"></a>
  <br>
  <a href="#installing"><img src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-507269" alt="Linux, Windows and macOS" height="20"></a>
  <a href="docs/DEVELOPMENT.md"><img src="https://img.shields.io/badge/core-Rust-555555?logo=rust" alt="Rust application core" height="20"></a>
  <a href="client/package.json"><img src="https://img.shields.io/badge/interface-Vue%20%2B%20Quasar-176d5e" alt="Vue and Quasar interface" height="20"></a>
  <a href="SECURITY.md"><img src="https://img.shields.io/badge/security-policy-507269" alt="Security policy and vulnerability reporting" height="20"></a>
</p>
<!-- project-badges:end -->

<hr>

This is the maintained [fantuz/onetagger](https://github.com/fantuz/onetagger) fork of
[Marekkon5/onetagger](https://github.com/Marekkon5/onetagger), with the original authors and license preserved.

OneTagger helps DJs enrich and organize the metadata in their own music files. AutoTagger can fetch metadata from providers including Beatport, Traxsource, Juno Download, Discogs, MusicBrainz and Spotify. The manual tag editor, keyboard-driven Quick Tag editor, and renamer support reviewing and organizing your collection.

Tag edits write to your audio files; retain backups and try a small selection before processing a collection. Online tagging contacts the selected providers, whose availability, accounts and rate limits may affect results. The inherited Spotify Audio Features page is currently unavailable; Spotify metadata tagging is a separate feature.

MP3, AIFF, WAV, FLAC, M4A (AAC, ALAC), and supported Ogg-family formats are handled by the tagging backend.

*For more info and tutorials check out the [upstream website](https://onetagger.github.io/).*

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4


## Installing

This fork has not published a tagged release yet. Download current builds from the [Build workflow on master](https://github.com/fantuz/onetagger/actions/workflows/build.yml?query=branch%3Amaster+event%3Apush): choose a successful run and open its **Artifacts** section. GitHub requires you to sign in to download workflow artifacts, which are retained for 14 days. Future tagged packages will appear on the [releases page](https://github.com/fantuz/onetagger/releases).

| Your computer | Artifact to download | Included packages |
| --- | --- | --- |
| Windows x64 | `onetagger-windows-x64` | Portable application, CLI, and `OneTagger-windows-setup.exe` installer |
| macOS Apple Silicon | `onetagger-macos-arm64` | Application bundle and CLI, in separate ZIP archives |
| macOS Intel | `onetagger-macos-intel` | Application bundle and CLI, in separate ZIP archives |
| Linux x64 | `onetagger-linux-x64` | Application and CLI, in separate tar.gz archives |

Extract the downloaded artifact ZIP to access the packages and its `SHA256SUMS-*.txt` file. Checksums detect download corruption; they are not a publisher signature. Windows packages are not Authenticode signed. macOS packages are not Developer ID signed or notarized; choose the archive matching your processor. Linux artifacts are built on Ubuntu 24.04 and require compatible glibc, ALSA and WebKitGTK 4.1 runtimes. The `frontend` artifact is an intermediate build output, not an installable application.

See [development and validation](docs/DEVELOPMENT.md) for the GitHub Actions pipeline, [safe file operations and browser sessions](docs/SAFETY.md) for file-handling details, and the [security policy](SECURITY.md) for trust boundaries and vulnerability reporting.


## Credits
Marekkon5 - Original OneTagger application<br>
Bas Curtiz - UI, Idea, Help  
SongRec (Shazam support) - https://github.com/marin-m/SongRec

## Support
Use this fork's [Discussions](https://github.com/fantuz/onetagger/discussions) for non-sensitive bugs, questions and feature requests. Include the build commit, operating system and a minimal reproduction. For suspected vulnerabilities, follow [SECURITY.md](SECURITY.md) and keep details private.

You can support the original upstream developers by donating on [PayPal](https://paypal.me/marekkon5) or [Patreon](https://www.patreon.com/onetagger)

## Building

On every platform, use the Rust toolchain pinned in `rust-toolchain.toml`, Node.js **24**, and pnpm **10.11.0**. Keep both lockfiles unchanged for a normal build.

### Linux

Install dependencies: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/package-manager/), [pnpm](https://pnpm.io/installation)

**Install remaining dependencies**
```
sudo apt install -y lld autogen libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.1-dev
```

**Compile UI**
```
cd client
pnpm install --frozen-lockfile
pnpm run build
cd ..
```

**Compile**
```
cargo build --workspace --release --locked
```
Output will be in: `target/release/onetagger`


### Windows
You need to install dependencies: [rustup](https://rustup.rs), [nodejs](https://nodejs.org/en/download/), [Visual Studio 2022 Build Tools](https://aka.ms/vs/17/release/vs_buildtools.exe), [pnpm](https://pnpm.io/installation)

**Compile UI:**
```
cd client
pnpm install --frozen-lockfile
pnpm run build
cd ..
```

**Compile OneTagger:**
```
cargo build --workspace --release --locked
```

Output will be inside `target\release` folder.


### macOS (Intel or Apple Silicon)

Install Rust with rustup, Node.js 24, pnpm 10.11.0, and Xcode Command Line Tools.
Build on the architecture you intend to run; liblzma is compiled from its locked crate source:

```sh
pnpm --dir client install --frozen-lockfile
pnpm --dir client run build
cargo build --workspace --release --locked
```

### Browser and server mode

The desktop window and `--browser` establish their session automatically. For a headless server:

```sh
onetagger-cli server --browser
```

Without `--browser`, open the session URL printed on the terminal. Each application restart creates a new session; stale browser tabs must use the new link. Do not publish that link. The default listener remains loopback-only. See [session security](docs/SAFETY.md#browser-sessions) before using `--expose`.
