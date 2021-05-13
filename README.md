# ![Logo](https://raw.githubusercontent.com/Marekkon5/onetagger/master/assets/32x32.png?token=ADTXOBSCCWLLZUBPGLOGXWDATVX6C) OneTagger

Cross-platform music tagger written in Rust, Vue.js and Quasar.
It can fetch metadata from Beatport, Traxsource, Juno Download and Discogs.
It is also able to fetch Spotify's Audio Features based on ISRC & exact match. 
There is a manual tag editor and quick tag editor which lets you use keyboard shortcuts.

MP3, AIFF & FLAC supported.

For more info check out our [website](https://onetagger.github.io/).

## Installing

You can download latest binaries from [releases](https://github.com/Marekkon5/onetagger/releases)

## Compilling

### Linux & Mac
Mac version is cross compiled from linux, the build script is designed to run on Github Actions enviromnent, so you have to install some dependencies manually: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/package-manager/)

**Install remaining dependencies**
```
sudo apt install -y autogen libsndfile1-dev libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.0-dev
```

**Compile Linux only**
```
cargo build --release
```
Output is in: `target/release/onetagger`

**Compile Linux + Mac, create bundles**
```
assets/compile-nix.sh
```
**Don't start the script directly or from assets dir. It has to be relative to the parent**  
Output files will be in the `dist/` directory.

### Windows
Build script is also designed to run on Github Actions environment, so you need to install some dependencies manually: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/), [python3](https://www.python.org/downloads/) (has to be accesible as `python`), [vcpkg](https://github.com/microsoft/vcpkg) (create enviromnent variable `VCPKG_ROOT` with path to it), Visual Studio Build Tools.  
7z and nsis can be installed using `chocolatey`
```
choco install nsis 7zip -y
```
(`nsis` is expected to be installed at default path in the script).

**Compile**
```
python assets/compile-win.py
```
**Don't start the script directly or from assets dir. It has to be relative to the parent**  
Output files will be in the `dist/` directory.  
**NOTE:** If your build on Windows fails, try running it inside `x64 Native Tools Command Prompt for VS`