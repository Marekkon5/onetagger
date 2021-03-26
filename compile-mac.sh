# Intended for Github Actions
# Requires Ubuntu, rustup and nodejs, npm installed
apt update
apt install -y pkg-config make yasm libssl-dev libxml2-dev cmake clang gcc g++ zlib1g-dev libmpc-dev libmpfr-dev libgmp-dev curl wget git python2
rustup target add x86_64-apple-darwin
# Compile UI
cd client
npm i
npm run build
cd ..
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
# Compile ffmpeg
wget https://ffmpeg.org/releases/ffmpeg-4.3.2.tar.gz
tar zxf ffmpeg-4.3.2.tar.gz
cd ffmpeg-4.3.2
./configure --prefix=x86_64-apple-darwin14 --disable-shared --enable-static --disable-ffplay --disable-doc --arch=x86_64 --target-os=darwin --cross-prefix=x86_64-apple-darwin14- --cc=o64-clang --prefix=$(dirname $(pwd))/target/SDK/MacOSX10.10.sdk/usr
make -j8
make install
# Setup ffmpeg
cd ..
export PKG_CONFIG_PATH="$PATH:$(pwd)/target/SDK/MacOSX10.10.sdk/usr/lib/pkgconfig"
cd ..
# Compile 1t
cargo build --target x86_64-apple-darwin --release