#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
PREFIX=${KF6_PREFIX:-"$ROOT/build/kf6-prefix"}
BUILD_ROOT=${KF6_BUILD_ROOT:-"$ROOT/build"}
DOWNLOADS="$BUILD_ROOT/downloads"
QT_PREFIX=${QT_ROOT_DIR:?QT_ROOT_DIR is not set}

ECM_VERSION=6.31.0
KIRIGAMI_VERSION=6.31.0
ECM_ARCHIVE="extra-cmake-modules-$ECM_VERSION.tar.xz"
KIRIGAMI_ARCHIVE="kirigami-$KIRIGAMI_VERSION.tar.xz"

mkdir -p "$DOWNLOADS" "$PREFIX"

fetch_archive() {
    name=$1
    checksum=$2
    url=$3
    file="$DOWNLOADS/$name"
    if [ ! -f "$file" ]; then
        curl -fL --retry 3 -o "$file" "$url"
    fi
    printf '%s  %s\n' "$checksum" "$file" | sha256sum -c -
}

fetch_archive \
    "$ECM_ARCHIVE" \
    8f74af329e4c918358bfe54316f8cf4d530c28444595a3e2fb15075f72877f17 \
    "https://download.kde.org/stable/frameworks/6.31/$ECM_ARCHIVE"
fetch_archive \
    "$KIRIGAMI_ARCHIVE" \
    811a8a150e636216269c0346e62470997fa282113a39e32b2d332bd5969926f0 \
    "https://download.kde.org/stable/frameworks/6.31/$KIRIGAMI_ARCHIVE"

tar -xJf "$DOWNLOADS/$ECM_ARCHIVE" -C "$BUILD_ROOT"
tar -xJf "$DOWNLOADS/$KIRIGAMI_ARCHIVE" -C "$BUILD_ROOT"

cmake -S "$BUILD_ROOT/extra-cmake-modules-$ECM_VERSION" \
    -B "$BUILD_ROOT/ecm-build" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DBUILD_TESTING=OFF
cmake --build "$BUILD_ROOT/ecm-build" --parallel
cmake --install "$BUILD_ROOT/ecm-build"

cmake -S "$BUILD_ROOT/kirigami-$KIRIGAMI_VERSION" \
    -B "$BUILD_ROOT/kirigami-build" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_PREFIX_PATH="$QT_PREFIX;$PREFIX" \
    -DBUILD_TESTING=OFF \
    -DBUILD_EXAMPLES=OFF \
    -DUSE_DBUS=OFF \
    -DKDE_INSTALL_BINDIR=bin \
    -DKDE_INSTALL_LIBDIR=lib \
    -DKDE_INSTALL_QMLDIR=lib/qt6/qml
cmake --build "$BUILD_ROOT/kirigami-build" --parallel
cmake --install "$BUILD_ROOT/kirigami-build"
