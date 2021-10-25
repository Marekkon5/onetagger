# Intended for Github Actions
# Requires Ubuntu, rustup and nodejs, npm installed
#sudo apt update
#sudo apt install -y autogen libsndfile1-dev libasound2-dev pkg-config make libssl-dev gcc g++ curl wget git libwebkit2gtk-4.0-dev
# Compile UI
cd client
npm i
npm run build
cd ..
# Compile for linux
cargo build --release
strip target/release/onetagger
tar zcf OneTagger-linux.tar.gz -C target/release onetagger
mkdir dist
mv OneTagger-linux.tar.gz dist/
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
# Install libsndfile
git clone https://github.com/libsndfile/libsndfile
cd libsndfile
git checkout d60deb5d8
./autogen.sh
./configure --disable-external-libs --enable-werror --host=x86_64-apple-darwin14 --prefix=$(dirname $(pwd))/target/SDK/MacOSX10.10.sdk/usr
make -j8
make install
cd ..
rm target/SDK/MacOSX10.10.sdk/usr/lib/libsndfile.*dylib
export PKG_CONFIG_PATH="$PATH:$(pwd)/target/SDK/MacOSX10.10.sdk/usr/lib/pkgconfig"
cd ..
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
cp target/x86_64-apple-darwin/release/bundle/osx/OneTagger-mac.zip dist/