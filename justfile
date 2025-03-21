# install build tools
install_deps:
    - sudo dnf install clang-devel glibc-devel
    cargo install cargo-zigbuild
    cargo install cargo-ndk

    # linux
    rustup target add x86_64-unknown-linux-gnu
    rustup target add aarch64-unknown-linux-gnu

    # windows
    rustup target add x86_64-pc-windows-gnu
    rustup target add aarch64-pc-windows-gnu

    # android
    rustup target add x86_64-linux-android
    rustup target add aarch64-linux-android

# standard local build
build:
    cargo build --release

# build all supported targets with zigbuild
release:
    cargo zigbuild --target x86_64-unknown-linux-gnu --release
    cargo zigbuild --target aarch64-unknown-linux-gnu --release
    cargo zigbuild --target x86_64-pc-windows-gnu --release
    cargo zigbuild --target aarch64-pc-windows-gnu --release

# build for Android
android:
    cargo ndk -t arm64-v8a build --release
    cargo ndk -t x86_64 build --release

# package output .so/.dll with .info into zip
package platform target ext:
    mkdir -p dist/{{platform}}
    cp target/{{target}}/release/libgametank_libretro.{{ext}} dist/{{platform}}/gametank_libretro.{{ext}}
    cp gametank_libretro.info dist/{{platform}}/
    cd dist/{{platform}} && zip ../gametank-core-{{platform}}.zip gametank_libretro.{{ext}} gametank_libretro.info

# package all builds
package-all:
    just package linux-x64 x86_64-unknown-linux-gnu so
    just package linux-arm64 aarch64-unknown-linux-gnu so
    just package win-x64 x86_64-pc-windows-gnu dll
    just package android-arm64 android/arm64-v8a so
