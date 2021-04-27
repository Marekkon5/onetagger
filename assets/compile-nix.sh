# Intended for Github Actions
# Requires Ubuntu, rustup and nodejs, npm installed
#sudo apt update
#sudo apt install -y libavcodec-dev libasound2-dev pkg-config make yasm libssl-dev libxml2-dev cmake clang gcc g++ zlib1g-dev libmpc-dev libmpfr-dev libgmp-dev curl wget git python2 ffmpeg libwebkit2gtk-4.0-dev
# Compile UI
cd client
npm i
npm run build
cd ..
# Compile for linux
cargo build --release
strip target/release/onetagger
# Mac
rustup target add x86_64-apple-darwin
# Install osxcross
git clone https://github.com/tpoechtrager/osxcross
cd osxcross
wget -nc https://s3.dockerproject.org/darwin/v2/MacOSX10.10.sdk.tar.xz
mv MacOSX10.10.sdk.tar.xz tarballs/
UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh
# Set variables
export PATH="$PATH:$(pwd)/target/bin"
export CC=o64-clang
export CXX=o64-clang++
export MACOSX_DEPLOYMENT_TARGET=10.8
export PKG_CONFIG_ALLOW_CROSS=1
# Compile 1t
cargo install cargo-bundle
cargo bundle --target x86_64-apple-darwin --release
x86_64-apple-darwin14-strip target/x86_64-apple-darwin/release/onetagger
# Create own zip with proper permissions
cd target/x86_64-apple-darwin/release/bundle/osx
x86_64-apple-darwin14-strip OneTagger.app/Contents/MacOS/onetagger
chmod +x OneTagger.app/Contents/MacOS/onetagger
zip -r OneTagger-mac.zip .
cd -
mkdir dist
cp target/x86_64-apple-darwin/release/bundle/osx/OneTagger-mac.zip dist/
cp target/release/onetagger dist/OneTagger-linux