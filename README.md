# OneTagger

## Compilling

### Linux & Mac
Mac version is cross compiled from linux, the build script is designed to run on Github Actions enviromnent, so you have to install some dependencies manually: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/package-manager/)

**Install remaining dependencies**
```
sudo apt install -y pkg-config make yasm libssl-dev libxml2-dev cmake clang gcc g++ zlib1g-dev libmpc-dev libmpfr-dev libgmp-dev curl wget git python2 ffmpeg libwebkit2gtk-4.0-dev
```

**Compile Linux only**
```
cargo run --release
```

**Compile Linux + Mac, create bundles**
```
assets/compile-nix.sh
```
**Don't start the script directly or from assets dir. It has to be relative to the parent**  
Output files will be in the `dist/` directory.

### Windows
Build script is also designed to run on Github Actions environment, so you need to install some dependencies manually: [rustup](https://rustup.rs), [node](https://nodejs.org/en/download/), [python3](https://www.python.org/downloads/) (has to be accesible as `python`), Visual Studio Build Tools.  
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