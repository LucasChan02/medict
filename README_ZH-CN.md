# Medict

[English](README.md) | **简体中文**

[![Latest Release](https://img.shields.io/github/v/release/LucasChan02/medict?display_name=tag&style=flat-square)](https://github.com/LucasChan02/medict/releases/latest)
[![Release](https://github.com/LucasChan02/medict/actions/workflows/package-and-release.yaml/badge.svg)](https://github.com/LucasChan02/medict/actions/workflows/package-and-release.yaml)
[![CI](https://github.com/LucasChan02/medict/actions/workflows/ci.yaml/badge.svg)](https://github.com/LucasChan02/medict/actions/workflows/ci.yaml)

Medict 是一款离线词典应用，支持 MDict 和 StarDict。当前应用使用 Rust
重写：

- `medict-core` 负责词典解析、搜索、HTML 渲染、生词本、ECDICT 下载、
  Anki 导出和词典资源服务。
- `medict-app` 使用 Qt Bridge 将 Rust 核心连接到 QML，不包含 C++ 应用代码，
  也不启动独立的后端进程。
- 界面使用 Kirigami、Plasma 控件、Breeze 图标和当前 KDE 配色方案。

当前版本为 **v4.0.3**。

## 功能

- MDict 1.x / 2.0 `.mdx` 和 `.mdd`。
- StarDict `.ifo`、`.idx`、`.dict`、`.dict.dz` 和 `.res`。
- 内置 ECDICT 英汉词典，可下载完整版。
- 精确、前缀、大小写变体和模糊查询。
- 单词典、多词典查询。
- Qt WebEngine 显示词典 CSS、JavaScript、图片、字体和音频。
- `entry://` 链接、`@@@LINK=` 跳转、双击查词和发音。
- 前进/后退历史和释义缩放。
- 词典目录扫描、符号链接选项和索引重建。
- 单词典 CSS 覆盖和实时预览。
- 生词本 HTML 快照和 Anki `.apkg` 导出。
- Kirigami 自适应界面和 KDE HIG 布局。
- 所有词典、数据库、网络和导出工作均在 Qt UI 线程之外运行。

## 下载

所有发布包由 GitHub Actions 构建，不使用本地发布构建。

| 平台 | 安装包 |
| --- | --- |
| Windows x86_64 | `Medict-4.0.3-windows-x86_64.exe` |
| Linux x86_64 | `Medict-4.0.3-linux-x86_64.AppImage` |
| Linux x86_64 | `Medict-4.0.3-linux-x86_64.flatpak` |

### Windows

直接运行便携 EXE。无需安装，也不需要管理员权限。单文件包含 Medict、Qt、
Kirigami、Qt WebEngine 和 MSVC 运行库。

### Linux AppImage

```bash
chmod +x Medict-4.0.3-linux-x86_64.AppImage
./Medict-4.0.3-linux-x86_64.AppImage
```

### Linux Flatpak

```bash
flatpak install Medict-4.0.3-linux-x86_64.flatpak
flatpak run io.github.terasum.Medict
```

Flatpak 使用 KDE 运行时和 Qt WebEngine BaseApp。

## 词典目录

一个目录视为一个词典。在 **Settings → Open dictionary folder** 打开词典目录，
添加文件后使用 **Refresh dictionaries**。

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

## 架构

QML/Kirigami 通过 Qt Bridge 调用 Rust 单例控制器。控制器把耗时操作交给
Rust 工作线程，并把 JSON 状态发给 QML。Qt WebEngine 通过仅绑定
`127.0.0.1`、带随机令牌的本地 HTTP 服务加载词典页面和资源。

Kirigami 本身是 QML 框架，因此界面声明必须使用 QML；应用逻辑和后端均为
Rust，项目不包含 C++ 应用源码。

## 发布

`VERSION` 使用三段式 `x.y.z`，同时作为 Cargo 工作区版本和发布标签。
版本级别规则请参阅 [VERSIONING.md](VERSIONING.md)。

1. 主版本必须取得维护者明确批准，次版本用于兼容功能，修订版本用于修复。
2. 更新 `VERSION`、`Cargo.toml`、`CHANGELOG.md`、AppStream 和工作流版本。
3. 推送匹配标签，例如 `v4.0.3`。
4. 等待 GitHub Actions 构建 Windows EXE、AppImage 和 Flatpak。
5. 检查 GitHub Release。

手动运行 Release 工作流会创建草稿 Release；推送标签会正式发布。

## 隐私

查词请求不会发送到互联网。词典资源服务只监听本机回环地址。完整 ECDICT
安装是唯一需要联网下载的数据功能。外部 HTTP/HTTPS 链接只由系统浏览器打开。

## 许可证

Medict 使用 [GNU General Public License v3.0 or later](LICENSE)。
