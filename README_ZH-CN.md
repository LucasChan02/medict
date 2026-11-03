# Medict

[English](README.md) | **简体中文**

[![Latest Release](https://img.shields.io/github/v/release/terasum/medict?display_name=tag&style=flat-square)](https://github.com/terasum/medict/releases/latest)
[![Release](https://github.com/terasum/medict/actions/workflows/package-and-release.yaml/badge.svg)](https://github.com/terasum/medict/actions/workflows/package-and-release.yaml)
[![CI](https://github.com/terasum/medict/actions/workflows/ci.yml/badge.svg)](https://github.com/terasum/medict/actions/workflows/ci.yml)
[![GitHub Stars](https://img.shields.io/github/stars/terasum/medict?style=flat-square)](https://github.com/terasum/medict/stargazers)
[![License](https://img.shields.io/github/license/terasum/medict.svg?style=flat-square)](LICENSE)

Medict 是一款离线词典应用，支持 MDict 和 StarDict 词典。桌面客户端使用
Qt 6、Kirigami 和 Qt WebEngine，并遵循 KDE 人机界面指南。在 KDE 上使用
当前 Plasma 样式和 Breeze 图标。本地 Go 进程读取词典文件，并提供控制 API。

当前稳定版本为 **v4.0.1**。

## 功能

### 界面

- 使用 Kirigami 全局抽屉和自适应页面布局。
- 在 KDE 上使用 Plasma 颜色、控件和 Breeze 图标。其他桌面使用原生
  Qt Quick 样式和内置 Breeze 图标。
- 提供独立的查词、生词本、设置、CSS 编辑器和关于页面。

### 查词

- 读取 MDict v1.x 和 v2.0 的 `.mdx` 文件及 `.mdd` 资源。
- 读取 StarDict 的 `.ifo`、`.idx`、`.dict` 和 `.dict.dz` 文件。
- 内置精简版 ECDICT 英汉词典，可离线使用。
- 可在设置中下载完整 ECDICT 数据。下载需要网络，安装后可离线查询。
- 支持前缀建议、模糊搜索和直接查词。
- 支持单词典查询和多词典同时查询。
- 自动保存当前词典选择。
- 使用 Qt WebEngine 显示词典 CSS、JavaScript、图片、字体和音频。
- 支持 `entry://` 链接和 `@@@LINK=` 跳转。
- 支持后退和前进历史。
- 支持 80% 到 160% 的释义缩放，并保存缩放设置。

### 词典工具

- 在启动时或按需扫描词典目录。
- 可选择是否跟随符号链接。
- 可重建单个词典的关键词索引。
- 可为每个词典编辑 CSS 覆盖样式。
- 在已打开的释义页面中实时预览 CSS 修改。
- 可将当前词条导出为独立 HTML 文件。

### 生词本

- 将当前单词保存到默认生词本。
- 创建、重命名和删除生词本。
- 设置默认生词本。
- 为每个收藏词条保存 HTML 快照。
- 从生词本直接打开收藏的单词。
- 将全部生词或单个生词本导出为 Anki `.apkg` 文件。

## 下载

请从 [GitHub Releases](https://github.com/terasum/medict/releases/latest)
下载最新版本。所有发布包由 GitHub Actions 构建。

| 平台 | 安装包 |
| --- | --- |
| Windows x86_64 | `Medict-4.0.1-windows-x86_64-setup.exe` |
| Linux x86_64 | `Medict-4.0.1-linux-x86_64.AppImage` |
| Linux x86_64 | `Medict-4.0.1-linux-x86_64.flatpak` |

### Windows

运行 `Medict-4.0.1-windows-x86_64-setup.exe`。安装程序不需要管理员权限。它会安装 Medict
和 `medictd.exe` 词典引擎。

### Linux AppImage

为 AppImage 添加执行权限，然后启动：

```bash
chmod +x Medict-4.0.1-linux-x86_64.AppImage
./Medict-4.0.1-linux-x86_64.AppImage
```

AppImage 包含 Medict 和 Go 词典引擎。它使用主机系统中的 Qt 6.10、
Qt WebEngine 和 Kirigami 库。启动前，请安装提供这些库的最新 KDE
Plasma 6 桌面环境。

部分 Linux 系统会禁用非特权用户命名空间。在这些系统上，释义区域可能为空。请
用以下命令启动 AppImage：

```bash
QTWEBENGINE_DISABLE_SANDBOX=1 ./Medict-4.0.1-linux-x86_64.AppImage
```

该命令会关闭 Qt WebEngine 沙箱。仅在默认启动失败时使用。

### Linux Flatpak

安装 Flatpak 包：

```bash
flatpak install Medict-4.0.1-linux-x86_64.flatpak
flatpak run io.github.terasum.Medict
```

Flatpak 包使用 KDE 运行时和 Qt WebEngine BaseApp。应用保留 Flatpak
沙箱和 Qt WebEngine 沙箱。

## 安装词典

Medict 将一个目录视为一个词典。打开 **Settings**，然后选择
**Open folder**。该按钮会打开当前系统的词典目录。

请将每个词典放入独立子目录：

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

Medict 会扫描子目录中的词典文件。添加文件后，请选择
**Dictionaries** 和 **Refresh dictionaries**。

### 词典文件规则

| 类型 | 必需文件 | 可选文件 |
| --- | --- | --- |
| MDict | 一个 `.mdx` 文件 | 同目录下的一个或多个 `.mdd` 文件 |
| StarDict | `.ifo`、`.idx` 和一个 `.dict` 或 `.dict.dz` 文件 | `cover.jpg` 或 `cover.png` |

词典内容和授权归各词典提供者所有。请仅使用已获授权的词典文件。

## 隐私与安全

Qt 客户端不会将查词请求发送到互联网。Go 引擎只监听 `127.0.0.1`，并在每次
启动时使用随机令牌。Qt 启动引擎，并在应用退出时停止引擎。

完整 ECDICT 安装需要联网下载词典数据。词典 HTML 可从引擎加载本地资源。
应用会在系统浏览器中打开外部 HTTP 或 HTTPS 链接。

## 开发

发布流程只使用 GitHub Actions。发布包由 `package-and-release.yaml`
工作流构建。仓库不使用本地构建产物发布版本。

### 组件

- `qt/`：Qt 6 Kirigami 客户端。
- `cmd/medictd/`：无界面 Go 词典引擎。
- `internal/desktopapi/`：受令牌保护的控制 API。
- `pkg/backserver/`：词典 HTTP 服务器。
- `packaging/`：Linux、Flatpak 和 Windows 打包文件。

### 架构

Qt 客户端以端口 `0` 启动 `medictd`。引擎绑定一个空闲的本地回环端口。
引擎输出一行 `MEDICT_READY` JSON，其中包含端口、静态 URL、API URL、
令牌、版本和配置目录。Qt 使用静态 URL 加载词典页面，并使用受令牌保护的
API 发送控制请求。

`medictd` 监听标准输入和退出信号。关闭应用时，Qt 客户端关闭标准输入管道。
随后引擎停止 HTTP 服务器，并关闭词典存储和生词本存储。

### 发布流程

1. 在 `VERSION` 中写入三段式版本号。构建和打包修复使用修订版本号升级，
   兼容功能使用次版本号升级，不兼容变更使用主版本号升级。同时更新
   `packaging/linux/io.github.terasum.Medict.metainfo.xml` 中对应的发布记录、
   `packaging/windows/medict.iss` 中的默认值和 `CHANGELOG.md`。发布前请对照
   上游 `terasum/medict` 的版本。
2. 推送与 `VERSION` 匹配的标签，例如 `v4.0.1`。
3. 等待 Release 工作流构建三个安装包。
4. 检查 GitHub Release。

手动运行工作流会创建草稿 Release。推送标签会发布 Release。发布文件使用
`Medict-<version>-<os>-<arch>.<format>` 命名。

## CI

CI 工作流执行以下检查：

- Go 构建、vet 和测试。
- 使用 Qt 6.10、Kirigami 6.31 和 Qt WebEngine 构建 Linux 版 Qt 客户端。
- 使用 Qt 6.10、Kirigami 6.31、MSVC 2022 和 Qt WebEngine 构建 Windows
  版 Qt 客户端。

## 路线图

- 在所有释义中进行全文检索。
- 为缺失词条增加词形和中文异体字回退。
- 继续优化大型词典的性能。
- 提供 MDX 打包和解包工具。
- 支持自定义资源解压目录。

当前工作请查看 [GitHub Issues](https://github.com/terasum/medict/issues)。

## 许可证

Medict 使用 [GNU General Public License v3.0](LICENSE)。
