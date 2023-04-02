#!/bin/bash
set -e

echo 'Building splash...'
flutter pub run flutter_native_splash:create

echo 'Generating serialization code...'
flutter pub run build_runner build

echo 'Generating Flutter Rust bridge code...'
# https://cjycode.com/flutter_rust_bridge/troubleshooting.html
export CPATH="$(clang -v 2>&1 | grep "Selected GCC installation" | rev | cut -d' ' -f1 | rev)/include"
flutter_rust_bridge_codegen --rust-input onetagger-android/src/api.rs --dart-output lib/api_generated.dart
unset CPATH

echo 'Compiling OneTagger...'
cd onetagger-android
# Fix aac lib compiling, because it links to system libraries on Android
export CFLAGS="-U__ANDROID__"
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 -p 26 -o ../android/app/src/main/jniLibs build --release
cd ..
