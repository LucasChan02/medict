# Medict

**English** | [简体中文](README_ZH-CN.md)

[![Latest Release](https://img.shields.io/github/v/release/LucasChan02/medict?display_name=tag&style=flat-square)](https://github.com/LucasChan02/medict/releases/latest)
[![Release](https://github.com/LucasChan02/medict/actions/workflows/package-and-release.yaml/badge.svg)](https://github.com/LucasChan02/medict/actions/workflows/package-and-release.yaml)
[![CI](https://github.com/LucasChan02/medict/actions/workflows/ci.yaml/badge.svg)](https://github.com/LucasChan02/medict/actions/workflows/ci.yaml)
[![License](https://img.shields.io/github/license/LucasChan02/medict.svg?style=flat-square)](LICENSE)

Medict is an offline dictionary for MDict and StarDict files. Version 5 is a
complete Rust rewrite:

- `medict-core` parses dictionaries, renders entries, stores notebooks,
  downloads ECDICT, exports Anki packages, and serves dictionary resources.
- `medict-app` binds the Rust core to QML through Qt Bridge. It contains no C++
  application code and starts no separate backend process.
- The QML interface uses Kirigami, Plasma controls, Breeze icons, and the
  active KDE color scheme.

The current release is **v5.0.0**.

## Features

- MDict 1.x and 2.0 `.mdx` files with `.mdd` resources.
- StarDict `.ifo`, `.idx`, `.dict`, `.dict.dz`, and `.res` files.
- Bundled ECDICT English-Chinese dictionary, with optional full ECDICT install.
- Exact, prefix, case-variant, and fuzzy lookup.
- Single-dictionary and multi-dictionary lookup.
- Dictionary CSS, JavaScript, images, fonts, and audio in Qt WebEngine.
- `entry://` links, `@@@LINK=` redirects, double-click lookup, and audio.
- Back and forward history and persistent definition zoom.
- Recursive dictionary scan, optional symlink following, and index rebuild.
- Per-dictionary CSS overrides with live preview and HTML entry export.
- Vocabulary notebooks with HTML snapshots and Anki `.apkg` export.
- Kirigami global drawer, adaptive pages, Plasma theme integration, and Breeze
  icon names.
- All dictionary, database, network, and export work runs off the Qt UI thread.

## Download

GitHub Actions builds every package. Local builds are not used for release
artifacts.

| Platform | Package |
| --- | --- |
| Windows x86_64 | `Medict-5.0.0-windows-x86_64-setup.exe` |
| Linux x86_64 | `Medict-5.0.0-linux-x86_64.AppImage` |
| Linux x86_64 | `Medict-5.0.0-linux-x86_64.flatpak` |

### Windows

Run the installer. It installs Medict, Qt, Kirigami, and Qt WebEngine. The
installer requests no administrator rights by default.

### Linux AppImage

```bash
chmod +x Medict-5.0.0-linux-x86_64.AppImage
./Medict-5.0.0-linux-x86_64.AppImage
```

The AppImage bundles the runtime libraries required to show the interface. On
KDE Plasma it also uses the host Plasma theme and icon integration.

### Linux Flatpak

```bash
flatpak install Medict-5.0.0-linux-x86_64.flatpak
flatpak run io.github.terasum.Medict
```

The Flatpak uses the KDE runtime and the Qt WebEngine BaseApp.

## Dictionaries

Medict treats one directory as one dictionary and scans subdirectories of the
dictionary folder. Open **Settings → Open dictionary folder** to find it.

```text
dicts/
├── Oxford/
│   ├── Oxford.mdx
│   ├── Oxford.mdd
│   └── Oxford.jpg
└── StarDictExample/
    ├── example.ifo
    ├── example.idx
    ├── example.dict.dz
    └── example.res/
```

Use **Refresh dictionaries** after adding files.

## Architecture

```text
QML / Kirigami
      │  properties, slots, signals
Qt Bridge singleton controller
      │
Rust `medict-core`
      ├── mdictlib: MDict parser
      ├── rusqlite: ECDICT + notebooks + Anki
      ├── Rust StarDict reader and fuzzy search
      ├── loopback content server with a random token
      └── worker threads for every blocking operation
```

The local HTTP server serves dictionary pages and resources to Qt WebEngine. It
binds only to `127.0.0.1` and uses a new random URL token for each process. The
control path is not HTTP: QML calls the Qt Bridge controller directly, and the
controller dispatches work to Rust workers.

Kirigami is a QML framework. A Kirigami interface therefore needs QML
declarations; the application and backend code is Rust, and the project has no
C++ application source.

## Release Process

`VERSION` is the single source for the workspace version and release tag. It
uses a three-segment `x.y.z` format.

1. Choose the change level:
   - major for architecture or incompatible behavior changes;
   - minor for compatible features;
   - patch for compatible fixes.
2. Update `VERSION`, `Cargo.toml`, `CHANGELOG.md`, the AppStream release entry,
   and the Inno Setup fallback.
3. Push a matching tag, for example `v5.0.0`.
4. The release workflow builds Windows x86_64, Linux x86_64 AppImage, and Linux
   x86_64 Flatpak packages.
5. Check the published GitHub Release.

A manual workflow run creates a draft release. A tag push publishes it. Asset
names follow `Medict-<version>-<os>-<arch>.<format>`.

## Development

The release path is GitHub Actions only:

- `.github/workflows/ci.yaml` checks the Rust workspace.
- `.github/workflows/package-and-release.yaml` creates release packages.

Requirements are Rust stable, Qt 6.10 or later with Qt Quick and Qt WebEngine,
KDE Frameworks with Kirigami, CMake, Ninja, and a supported C++ toolchain. Qt
Bridge needs `qmake` in `PATH`.

## Privacy

Medict does not send lookup requests to the internet. The content server is
loopback-only. The full ECDICT install is the only feature that downloads data.
Dictionary pages can open external HTTP or HTTPS links only through the system
browser.

## License

Medict uses the [GNU General Public License v3.0 or later](LICENSE).
