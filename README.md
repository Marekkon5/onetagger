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

_For more info and tutorials check out our [website](https://onetagger.github.io/)._

https://user-images.githubusercontent.com/15169286/193469224-cbf3af71-f6d7-4ecd-bdbf-5a1dca2d99c8.mp4

## Installing

You can download latest binaries from [releases](https://github.com/Marekkon5/onetagger/releases)

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
