# Medict

**English** | [简体中文](README_ZH-CN.md)

[![Latest Release](https://img.shields.io/github/v/release/terasum/medict?display_name=tag&style=flat-square)](https://github.com/terasum/medict/releases/latest)
[![Release](https://github.com/terasum/medict/actions/workflows/package-and-release.yaml/badge.svg)](https://github.com/terasum/medict/actions/workflows/package-and-release.yaml)
[![CI](https://github.com/terasum/medict/actions/workflows/ci.yml/badge.svg)](https://github.com/terasum/medict/actions/workflows/ci.yml)
[![GitHub Stars](https://img.shields.io/github/stars/terasum/medict?style=flat-square)](https://github.com/terasum/medict/stargazers)
[![License](https://img.shields.io/github/license/terasum/medict.svg?style=flat-square)](LICENSE)

Medict is an offline dictionary application. It reads MDict and StarDict
dictionaries. The desktop client uses Qt 6 Widgets and Qt WebEngine. A local
Go process reads the dictionary files and supplies the control API.

The current stable release is **v4.0.0**.

## Features

### Lookup

- Reads MDict v1.x and v2.0 `.mdx` files and their `.mdd` resources.
- Reads StarDict `.ifo`, `.idx`, `.dict`, and `.dict.dz` files.
- Includes a compact offline ECDICT English-Chinese dictionary.
- Downloads the complete ECDICT data from Settings. The download needs network
  access. Lookup remains offline after the install.
- Provides prefix suggestions, fuzzy search, and direct lookup.
- Looks up a word in one dictionary or in all selected dictionaries.
- Saves the active dictionary selection between sessions.
- Shows dictionary CSS, JavaScript, images, fonts, and audio in Qt WebEngine.
- Follows `entry://` links and `@@@LINK=` redirects.
- Keeps back and forward history.
- Changes definition zoom from 80% to 160% and saves the setting.

### Dictionary tools

- Scans the dictionary folder at startup and on demand.
- Optionally follows symbolic links.
- Rebuilds the keyword index for one dictionary.
- Edits a CSS override for each dictionary.
- Previews CSS changes in the open definition pages.
- Exports the current entry as a self-contained HTML file.

### Vocabulary notebooks

- Saves the current word to the default notebook.
- Creates, renames, and deletes notebooks.
- Sets the default notebook.
- Keeps an HTML snapshot of each saved entry.
- Opens a saved word directly from the notebook.
- Exports all words or one notebook to Anki `.apkg` format.

## Download

Download the latest release from
[GitHub Releases](https://github.com/terasum/medict/releases/latest).
GitHub Actions builds every release package.

| Platform | Package |
| --- | --- |
| Windows x86_64 | `Medict-4.0.0-Setup.exe` |
| Linux x86_64 | `Medict-4.0.0-x86_64.AppImage` |
| Linux x86_64 | `Medict-4.0.0-x86_64.flatpak` |

### Windows

Run `Medict-4.0.0-Setup.exe`. The installer does not require administrator
rights. It installs Medict and the `medictd.exe` dictionary engine.

### Linux AppImage

Make the AppImage executable and run it:

```bash
chmod +x Medict-4.0.0-x86_64.AppImage
./Medict-4.0.0-x86_64.AppImage
```

The AppImage includes Qt, Qt WebEngine, and the Go dictionary engine.
KDE Plasma is the primary Linux desktop target. Other Qt-capable desktops can
start the application.

Some Linux systems block unprivileged user namespaces. On these systems, the
definition area can stay empty. Start the AppImage with this command:

```bash
QTWEBENGINE_DISABLE_SANDBOX=1 ./Medict-4.0.0-x86_64.AppImage
```

This command disables the Qt WebEngine sandbox. Use it only when the default
start fails.

### Linux Flatpak

Install the Flatpak package:

```bash
flatpak install Medict-4.0.0-x86_64.flatpak
flatpak run io.github.terasum.Medict
```

The Flatpak uses the KDE runtime and the Qt WebEngine BaseApp. It keeps the
Flatpak sandbox and the Qt WebEngine sandbox.

## Install Dictionaries

Medict treats one directory as one dictionary. Open **Settings**, then select
**Open folder**. The button opens the dictionary directory for the current
system.

Put each dictionary in its own subdirectory:

```text
dicts/
├── Oxford/
│   ├── Oxford.mdx
│   ├── Oxford.mdd
│   └── Oxford.jpg
└── StarDictExample/
    ├── example.ifo
    ├── example.idx
    └── example.dict.dz
```

Medict scans subdirectories for dictionary files. Use **Dictionaries**,
**Refresh dictionaries** after you add files.

### Dictionary File Rules

| Type | Required files | Optional files |
| --- | --- | --- |
| MDict | One `.mdx` file | One or more `.mdd` files in the same directory |
| StarDict | `.ifo`, `.idx`, and one `.dict` or `.dict.dz` file | `cover.jpg` or `cover.png` |

Dictionary content and licensing belong to each dictionary provider. Use only
dictionary files that you have permission to use.

## Privacy and Security

The Qt client does not send search requests to the internet. The Go engine
listens only on `127.0.0.1` and uses a random token for each start. Qt starts
the engine and stops it when the application exits.

The complete ECDICT install uses network access to download dictionary data.
Dictionary HTML can load local resources from the engine. The application
opens external HTTP or HTTPS links in the system browser.

## Development

The release path uses GitHub Actions. Release packages come from the
`package-and-release.yaml` workflow. The repository does not use local builds
for release artifacts.

### Components

- `qt/`: Qt 6 Widgets client.
- `cmd/medictd/`: headless Go dictionary engine.
- `internal/desktopapi/`: token-protected control API.
- `pkg/backserver/`: dictionary HTTP server.
- `packaging/`: Linux, Flatpak, and Windows package files.

### Requirements

- Go 1.25 or later.
- Qt 6.8 or later with Widgets, Network, and WebEngine Widgets.
- CMake 3.24 or later.
- Windows with Visual Studio 2022, or Linux with a C++17 compiler.

### Architecture

The Qt client starts `medictd` with port `0`. The engine binds to a free
loopback port. It prints one `MEDICT_READY` JSON line with the port, static
URL, API URL, token, version, and configuration directory. Qt uses the static
URL for dictionary pages and the token-protected API for control requests.

`medictd` watches standard input and exit signals. The Qt client closes the
standard input pipe during shutdown. The engine then stops the HTTP server and
closes the dictionary and notebook stores.

### Release Process

1. Update the version in these files:
   `qt/CMakeLists.txt`, `packaging/windows/medict.iss`,
   `packaging/flatpak/io.github.terasum.Medict.yaml`,
   `packaging/linux/io.github.terasum.Medict.metainfo.xml`, and `CHANGELOG.md`.
   The workflow passes the tag version to the Windows and AppImage builds.
2. Push a tag such as `v4.0.0`.
3. Wait for the Release workflow to build the three packages.
4. Check the GitHub Release.

A manual workflow run creates a draft release. A tag push publishes the
release.

## CI

The CI workflow runs these checks:

- Go build, vet, and tests.
- Qt Linux build with Qt 6.8 and Qt WebEngine.
- Qt Windows build with Qt 6.8, MSVC 2022, and Qt WebEngine.

## Roadmap

- Full-text search in all definitions.
- Inflection and Chinese variant fallback for missing entries.
- More performance work for large dictionaries.
- MDX packing and unpacking tools.
- A configurable resource extraction directory.

See [GitHub Issues](https://github.com/terasum/medict/issues) for current work.

## License

Medict uses the [GNU General Public License v3.0](LICENSE).
