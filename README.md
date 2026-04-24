# Disco

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/disco.svg)](https://crates.io/crates/disco)

A powerful CLI tool for local multi-disk storage scheduling and management.

[中文文档](#中文文档)

## Features

- **Disk Pool Management** - Organize multiple independent disks into a unified storage pool
- **Offline Index Search** - Build file indexes with offline fuzzy search capability
- **Smart Storage Scheduling** - Automatically plan file storage locations with multiple allocation strategies
- **Solid/SolidLayer Rules** - Mark directories as Solid (inseparable) or set layered splitting depth
- **Terminal Visualization** - TUI interface for disk status and file distribution
- **Interactive Shell** - Interactive command input with menu navigation
- **Multi-language Support** - English and Simplified Chinese

## Installation

### One-line Install (Apple Silicon Mac)

```bash
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh
```

This script will:
- Detect Apple Silicon Mac (arm64)
- Install Rust if not present
- Compile and install Disco
- Add to PATH automatically

For other options:
```bash
# Show help
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh -s -- --help

# Custom install path
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh -s -- --prefix /usr/local/bin

# Uninstall
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh -s -- --uninstall
```

### One-line Install (Windows)

```powershell
irm https://raw.githubusercontent.com/Dujddx/disco/main/install.ps1 | iex
```

This script will:
- Detect Windows x64
- Install Rust via rustup if not present
- Compile and install Disco
- Add to PATH automatically

For other options:
```powershell
# Show help
.\install.ps1 -Help

# Custom install path
.\install.ps1 -Prefix "C:\Tools\Disco"

# Uninstall
.\install.ps1 -Uninstall
```

### One-line Install (Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh
```

This script will:
- Detect Linux distribution and architecture
- Install build dependencies (gcc, make, git, curl)
- Install Rust via rustup if not present
- Compile and install Disco
- Add to PATH automatically

Supported distributions:
- Debian/Ubuntu (apt)
- Fedora/RHEL/CentOS (dnf/yum)
- Arch Linux (pacman)
- openSUSE (zypper)
- Alpine (apk)

For other options:
```bash
# Show help
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh -s -- --help

# Custom install path
./install-linux.sh --prefix /usr/local/bin

# Uninstall
./install-linux.sh --uninstall
```

### From Source

```bash
git clone https://github.com/Dujddx/disco.git
cd disco
cargo build --release
```

The binary will be available at `target/release/disco`.

### Using Cargo Install

```bash
cargo install --path .
```

## Quick Start

### 1. Add a Disk to the Pool

```bash
disco disk add /Volumes/MyDisk
```

### 2. Scan and Index Files

```bash
disco scan --all
```

### 3. Search for Files

```bash
disco search "keyword"
```

### 4. Store Files to the Disk Pool

```bash
disco store /path/to/folder
```

### 5. Retrieve Files

```bash
disco retrieve <entry-id>
```

## Commands

### Disk Management

| Command | Description |
|---------|-------------|
| `disco disk add <mount-point>` | Add a disk to the pool |
| `disco disk list` | List all registered disks |

### Index & Search

| Command | Description |
|---------|-------------|
| `disco scan --all` | Scan and index all disks |
| `disco scan --disk <name>` | Scan a specific disk |
| `disco search <keyword>` | Search files in the index |
| `disco get <entry-id>` | Get file location by ID |

### Storage & Retrieval

| Command | Description |
|---------|-------------|
| `disco store <paths>` | Store files/folders to the disk pool |
| `disco retrieve <entry-id>` | Retrieve files from the disk pool |
| `disco solid set <path>` | Mark directory as Solid (inseparable) |
| `disco solid unset <path>` | Remove Solid marking from directory |

### Visualization & Configuration

| Command | Description |
|---------|-------------|
| `disco visualize` | Open terminal visualization interface |
| `disco menu` | Open menu navigation mode |
| `disco config lang [code]` | Set/view display language |

### Interactive Mode

Run `disco` or `disco -i` to enter interactive shell mode.

## Storage Options

### SolidLayer Depth

Use the `--solid-layer` parameter to control directory splitting depth:

- `0` - No splitting, treat entire directory as atomic unit
- `1` - Split to first-level subdirectories
- `2` - Split to second-level subdirectories
- `inf` - Split to file level

### Other Options

- `--dedup` - Enable hash-based deduplication
- `--preview` - Preview storage plan without executing
- `--yes` - Skip confirmation prompts

## Configuration

Configuration files are stored in:

- **macOS**: `~/Library/Application Support/disco/`
- **Linux**: `~/.config/disco/`
- **Windows**: `%APPDATA%\disco\`

The database file `disco.db` is stored in the configuration directory.

## Tech Stack

- **Rust** - Programming language
- **SQLite** - Data persistence
- **ratatui** - TUI component library
- **clap** - CLI argument parsing
- **blake3** - File hashing

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

# 中文文档

本地多磁盘存储调度 CLI 工具。

## 功能特性

- **磁盘池管理** - 将多个独立磁盘组织成统一的磁盘池，统一管理
- **离线索引搜索** - 扫描磁盘文件构建索引，支持离线模糊搜索
- **智能存储调度** - 自动规划文件存储位置，支持多种分配策略
- **Solid/SolidLayer 规则** - 标记目录为 Solid（不可分割）或设置分层分割深度
- **终端可视化** - TUI 界面展示磁盘状态和文件分布
- **交互式 Shell** - 支持交互式命令输入和菜单导航
- **多语言支持** - 支持英文和简体中文

## 安装

### 一键安装 (Apple Silicon Mac)

```bash
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh
```

### 一键安装 (Windows)

```powershell
irm https://raw.githubusercontent.com/Dujddx/disco/main/install.ps1 | iex
```

### 一键安装 (Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh
```

支持的发行版：Debian/Ubuntu、Fedora/RHEL、Arch Linux、openSUSE、Alpine

### 从源码编译

```bash
cargo build --release
```

或使用 `cargo install` 安装到本地：

```bash
cargo install --path .
```

## 快速开始

### 1. 添加磁盘到磁盘池

```bash
disco disk add /Volumes/MyDisk
```

### 2. 扫描磁盘建立索引

```bash
disco scan --all
```

### 3. 搜索文件

```bash
disco search "关键词"
```

### 4. 存储文件到磁盘池

```bash
disco store /path/to/folder
```

### 5. 检索文件

```bash
disco retrieve <entry-id>
```

## 命令列表

### 磁盘管理

| 命令 | 说明 |
|------|------|
| `disco disk add <mount-point>` | 添加磁盘到磁盘池 |
| `disco disk list` | 列出所有已注册磁盘 |

### 索引与搜索

| 命令 | 说明 |
|------|------|
| `disco scan --all` | 扫描所有磁盘建立/更新索引 |
| `disco scan --disk <name>` | 扫描指定磁盘 |
| `disco search <keyword>` | 在索引中搜索文件 |
| `disco get <entry-id>` | 根据 ID 获取文件位置 |

### 存储与检索

| 命令 | 说明 |
|------|------|
| `disco store <paths>` | 将文件/文件夹存储到磁盘池 |
| `disco retrieve <entry-id>` | 从磁盘池检索文件 |
| `disco solid set <path>` | 标记目录为 Solid（不可分割） |
| `disco solid unset <path>` | 移除目录的 Solid 标记 |

### 可视化与配置

| 命令 | 说明 |
|------|------|
| `disco visualize` | 打开终端可视化界面 |
| `disco menu` | 打开菜单导航模式 |
| `disco config lang [code]` | 设置/查看显示语言 |

### 交互模式

直接运行 `disco` 或 `disco -i` 进入交互式 Shell 模式。

## 存储选项

### SolidLayer 深度

使用 `--solid-layer` 参数控制目录分割深度：

- `0` - 不分割，整个目录作为原子单元
- `1` - 分割到第一级子目录
- `2` - 分割到第二级子目录
- `inf` - 分割到文件级别

### 其他选项

- `--dedup` - 启用基于哈希的去重
- `--preview` - 预览存储计划而不执行
- `--yes` - 跳过确认提示

## 配置文件

配置文件位于：
- macOS: `~/Library/Application Support/disco/`
- Linux: `~/.config/disco/`
- Windows: `%APPDATA%\disco\`

数据库文件 `disco.db` 存储在配置目录中。

## 技术栈

- **Rust** - 编程语言
- **SQLite** - 数据持久化
- **ratatui** - TUI 组件库
- **clap** - CLI 参数解析
- **blake3** - 文件哈希

## 许可证

MIT License
