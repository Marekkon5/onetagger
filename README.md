<p align='center'>
    <img alt='Logo' src='assets/onetagger-logo-github.png'>
</p>
<h1 align='center'>The ultimate cross-platform tagger for DJs</h1>

<h3 align='center'><b>
<a href='#installing'>Downloads</a> · <a href='CHANGELOG.md'>Release notes</a> · <a href='#building'>Build from source</a> · <a href='https://onetagger.github.io/'>Upstream tutorials</a> · <a href='#support'>Support</a>
</b></h3>

<!-- project-badges:start -->
<p align="center">
  <!-- Build and release availability -->
  <a href="https://github.com/fantuz/onetagger/actions/workflows/build.yml?query=branch%3Amaster"><img src="https://github.com/fantuz/onetagger/actions/workflows/build.yml/badge.svg?branch=master" alt="Tests and packages" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/actions/workflows/github-code-scanning/codeql?query=branch%3Amaster"><img src="https://github.com/fantuz/onetagger/actions/workflows/github-code-scanning/codeql/badge.svg?branch=master" alt="CodeQL analysis workflow" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/releases/latest"><img src="https://img.shields.io/github/v/release/fantuz/onetagger?label=release" alt="Latest stable release" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/releases"><img src="https://img.shields.io/github/downloads/fantuz/onetagger/total?label=downloads" alt="GitHub release downloads" height="20"></a>
  <br>
  <!-- License and repository activity -->
  <a href="LICENSE"><img src="https://img.shields.io/github/license/fantuz/onetagger" alt="GNU GPL v3 license" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/stargazers"><img src="https://img.shields.io/github/stars/fantuz/onetagger?style=flat" alt="GitHub stars" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/forks"><img src="https://img.shields.io/github/forks/fantuz/onetagger?style=flat" alt="GitHub forks" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/commits/master"><img src="https://img.shields.io/github/last-commit/fantuz/onetagger/master" alt="Last commit on master" height="20"></a>
  <br>
  <!-- Platforms and application components -->
  <a href="#installing"><img src="https://img.shields.io/badge/platforms-Linux%20%7C%20Windows%20%7C%20macOS-507269" alt="Linux, Windows and macOS" height="20"></a>
  <a href="docs/DEVELOPMENT.md"><img src="https://img.shields.io/badge/core-Rust-555555?logo=rust" alt="Rust application core" height="20"></a>
  <a href="client/package.json"><img src="https://img.shields.io/badge/interface-Vue%20%2B%20Quasar-176d5e" alt="Vue and Quasar interface" height="20"></a>
  <a href="#installing"><img src="https://img.shields.io/badge/packages-desktop%20%2B%20CLI-2e7d32" alt="Desktop application and command-line packages" height="20"></a>
  <br>
  <!-- Contributions and support; this fork uses Discussions instead of Issues -->
  <a href="https://github.com/fantuz/onetagger/pulls"><img src="https://img.shields.io/github/issues-pr/fantuz/onetagger?label=pull%20requests" alt="Open pull requests" height="20"></a>
  <a href="https://github.com/fantuz/onetagger/discussions"><img src="https://img.shields.io/badge/discussions-community-507269" alt="Community discussions" height="20"></a>
  <a href="SECURITY.md"><img src="https://img.shields.io/badge/security-policy-507269" alt="Security policy and vulnerability reporting" height="20"></a>
</p>
<!-- project-badges:end -->

<hr>

This is the maintained [fantuz/onetagger](https://github.com/fantuz/onetagger) fork of
[Marekkon5/onetagger](https://github.com/Marekkon5/onetagger), with the original authors and license preserved.

OneTagger helps DJs enrich and organize the metadata in their own music files. AutoTagger can fetch metadata from providers including Beatport, Traxsource, Juno Download, Discogs, MusicBrainz and Spotify. The manual tag editor, keyboard-driven Quick Tag editor, and renamer support reviewing and organizing your collection.

<!-- platform-downloads:start -->
<p align="center">
  <a href="https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-windows-setup.exe"><img src="https://img.shields.io/badge/Download%20for%20Windows-x64%20installer-0078D4?style=for-the-badge" alt="Download OneTagger for Windows — x64 installer" height="28"></a>
</p>
<p align="center">
  <a href="https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-arm64.zip"><img src="https://img.shields.io/badge/Download%20for%20macOS-Apple%20Silicon%20ARM64-222222?style=for-the-badge" alt="Download OneTagger for macOS — Apple Silicon ARM64 application ZIP" height="28"></a>
</p>
<p align="center">
  <a href="https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-intel.zip"><img src="https://img.shields.io/badge/Download%20for%20macOS-Intel%20x86--64-555555?style=for-the-badge" alt="Download OneTagger for macOS — Intel x86-64 application ZIP" height="28"></a>
</p>
<p align="center">
  <a href="https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-linux-x64.tar.gz"><img src="https://img.shields.io/badge/Download%20for%20Linux-x86--64%20tar.gz-2e7d32?style=for-the-badge" alt="Download OneTagger for Linux — x86-64 application tar.gz" height="28"></a>
</p>
<!-- platform-downloads:end -->

<p align="center">
  <a href="#installing">Portable Windows EXE, CLI downloads and checksums</a> · <a href="https://github.com/fantuz/onetagger/releases/latest">Latest release notes</a> · <a href="https://github.com/fantuz/onetagger/releases">All releases</a>
</p>

The buttons download the desktop application from the latest stable release. Choose the package matching your operating system and processor; see [installation details](#installing) below.

Tag edits write to your audio files; retain backups and try a small selection before processing a collection. Online tagging contacts the selected providers, whose availability, accounts and rate limits may affect results. The inherited Spotify Audio Features page is currently unavailable; Spotify metadata tagging is a separate feature.

MP3, AIFF, WAV, FLAC, M4A (AAC, ALAC), and supported Ogg-family formats are handled by the tagging backend.

## Installing

Download the [latest release](https://github.com/fantuz/onetagger/releases/latest) for your computer. Published packages and checksums are available without signing in to GitHub and do not expire like workflow artifacts.

| Your computer | Desktop application | Command-line tool | SHA-256 |
| --- | --- | --- | --- |
| Windows x64 | [Installer](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-windows-setup.exe) · [Portable EXE](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-windows.exe) | [CLI EXE](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-windows-cli.exe) | [Checksums](https://github.com/fantuz/onetagger/releases/latest/download/SHA256SUMS-windows-x64.txt) |
| macOS Apple Silicon (ARM64) | [Application ZIP](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-arm64.zip) | [CLI ZIP](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-arm64-cli.zip) | [Checksums](https://github.com/fantuz/onetagger/releases/latest/download/SHA256SUMS-macos-arm64.txt) |
| macOS Intel (x86-64) | [Application ZIP](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-intel.zip) | [CLI ZIP](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-macos-intel-cli.zip) | [Checksums](https://github.com/fantuz/onetagger/releases/latest/download/SHA256SUMS-macos-intel.txt) |
| Linux x64 (x86-64) | [Application tar.gz](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-linux-x64.tar.gz) | [CLI tar.gz](https://github.com/fantuz/onetagger/releases/latest/download/OneTagger-linux-x64-cli.tar.gz) | [Checksums](https://github.com/fantuz/onetagger/releases/latest/download/SHA256SUMS-linux-x64.txt) |

Extract ZIP/tar.gz packages before use. Each release includes a `SHA256SUMS-*.txt` file for each platform. Checksums detect download corruption; they are not a publisher signature. Windows packages are not Authenticode signed. macOS packages are not Developer ID signed or notarized; choose the archive matching your processor. Linux packages are built on Ubuntu 24.04 and require compatible glibc, ALSA and WebKitGTK 4.1 runtimes.

The Windows installer checks for the Microsoft Edge WebView2 Runtime and runs
the bootstrapper when it is missing. The portable Windows desktop executable
needs WebView2 installed on the system.

If the Linux native window freezes while browser mode works, launch `./onetagger --server --browser` to use the same local backend in your browser. This avoids the native WebKitGTK display path. Include your distribution, WebKitGTK version, GPU and Wayland/X11 session when reporting a native-window issue; browser success alone does not identify its exact cause.

For newer development builds, use the [Build workflow on master](https://github.com/fantuz/onetagger/actions/workflows/build.yml?query=branch%3Amaster+event%3Apush): choose a successful run and open its **Artifacts** section. Download the `onetagger-` artifact matching your platform and extract its outer ZIP to access the packages and checksums. GitHub requires sign-in for workflow artifacts, which are retained for 14 days. The `frontend` artifact is an intermediate build output, not an installable application.

Reviewed version-bump PRs trigger publication after the full GitHub build passes. Maintainers can also prepare a draft or retry publication through the [release workflow](https://github.com/fantuz/onetagger/actions/workflows/release.yml); see [release preparation and recovery](docs/DEVELOPMENT.md#artifacts-and-releases).

See [development and validation](docs/DEVELOPMENT.md) for the GitHub Actions pipeline, [safe file operations and browser sessions](docs/SAFETY.md) for file-handling details, and the [security policy](SECURITY.md) for trust boundaries and vulnerability reporting.

## Related projects

[rekordbox Migration Helper · Library Workbench](https://github.com/fantuz/rekordbox-migration-helper) complements OneTagger with library inspection, duplicate review and migration preparation for rekordbox collections.

Its [OneTagger bridge](https://github.com/fantuz/rekordbox-migration-helper/blob/main/docs/AUDIO-MATCHING-AND-ONETAGGER.md#onetagger-handoff) exports a JSON selection manifest or M3U of local music files. Load the manifest in **Auto tag → Input & Tags** to save a results JSON with before/after tags and provider evidence, then import it into the same Helper selection for review. The Helper independently verifies the files and stages only the fields you select. [Bridge instructions and report format](docs/ONETAGGER-RESULTS.md).

Run both tools on the same computer. OneTagger writes the audio-file tags; the bridge stages approved fields in Workbench while preserving personal stars, cues and original file references. The original tag-rescan workflow remains available.

## Preview and tutorials

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4

Visit the [upstream website](https://onetagger.github.io/) for tutorials and feature guides. The video shows the upstream interface; use this fork's release links above for maintained downloads.


## Credits
Marekkon5 - Original OneTagger application<br>
Bas Curtiz - UI, Idea, Help<br>
Build pipelines and security audit by [Massimiliano Fantuzzi](https://github.com/fantuz)<br>
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
