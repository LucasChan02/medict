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
MARKER="$PREFIX/.medict-kirigami-$KIRIGAMI_VERSION"

mkdir -p "$DOWNLOADS" "$PREFIX"

phase() {
    printf '[%s] %s\n' "$(date -u +%H:%M:%S)" "$*"
}

test_kirigami_install() {
    qml_dir="$PREFIX/lib/qt6/qml/org/kde/kirigami"
    test -f "$qml_dir/qmldir" &&
        find "$qml_dir" -maxdepth 1 -type f -name '*plugin*.so' -print -quit |
            grep -q . &&
        find "$PREFIX/lib" -maxdepth 1 \( -type f -o -type l \) \
            \( -name 'libKirigami*.so*' -o -name 'libKF6Kirigami*.so*' \) \
            -print -quit | grep -q .
}

if [ -f "$MARKER" ]; then
    if test_kirigami_install; then
        phase "Reusing installed Kirigami $KIRIGAMI_VERSION from $PREFIX."
        exit 0
    fi
    phase "Cached Kirigami install is incomplete. Rebuilding it."
    rm -f "$MARKER"
fi

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

phase "Downloading ECM and Kirigami"
fetch_archive \
    "$ECM_ARCHIVE" \
    8f74af329e4c918358bfe54316f8cf4d530c28444595a3e2fb15075f72877f17 \
    "https://download.kde.org/stable/frameworks/6.31/$ECM_ARCHIVE"
fetch_archive \
    "$KIRIGAMI_ARCHIVE" \
    811a8a150e636216269c0346e62470997fa282113a39e32b2d332bd5969926f0 \
    "https://download.kde.org/stable/frameworks/6.31/$KIRIGAMI_ARCHIVE"

phase "Extracting ECM and Kirigami"
tar -xJf "$DOWNLOADS/$ECM_ARCHIVE" -C "$BUILD_ROOT"
tar -xJf "$DOWNLOADS/$KIRIGAMI_ARCHIVE" -C "$BUILD_ROOT"

phase "Configuring ECM"
cmake -S "$BUILD_ROOT/extra-cmake-modules-$ECM_VERSION" \
    -B "$BUILD_ROOT/ecm-build" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DBUILD_TESTING=OFF
phase "Building ECM"
cmake --build "$BUILD_ROOT/ecm-build" --parallel
phase "Installing ECM"
cmake --install "$BUILD_ROOT/ecm-build"

phase "Configuring Kirigami"
cmake -S "$BUILD_ROOT/kirigami-$KIRIGAMI_VERSION" \
    -B "$BUILD_ROOT/kirigami-build" \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_PREFIX_PATH="$QT_PREFIX;$PREFIX" \
    -DBUILD_TESTING=OFF \
    -DBUILD_EXAMPLES=OFF \
    -DBUILD_SHARED_LIBS=ON \
    -DUSE_DBUS=OFF \
    -DKDE_INSTALL_BINDIR=bin \
    -DKDE_INSTALL_LIBDIR=lib \
    -DKDE_INSTALL_QMLDIR=lib/qt6/qml
phase "Building Kirigami"
cmake --build "$BUILD_ROOT/kirigami-build" --parallel
phase "Installing Kirigami"
cmake --install "$BUILD_ROOT/kirigami-build"
if ! test_kirigami_install; then
    phase "Kirigami install is missing its QML module or runtime libraries in $PREFIX."
    exit 1
fi
touch "$MARKER"
phase "Kirigami $KIRIGAMI_VERSION is ready."
