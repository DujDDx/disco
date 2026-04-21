# Plan: Interactive CLI Mode + Disk Offline Bug Fix

## Context

Disco 的 CLI 目前只支持单次命令执行。用户需要一个交互式 shell 模式（`-i`/`--interactive`），在其中可以执行所有操作而无需重复输入 `disco` 前缀。同时，磁盘经常误报 offline 的根本原因是 fingerprint 匹配逻辑有 bug——检测时用 `Utc::now()` 生成 fingerprint，永远无法匹配注册时的 fingerprint。交互模式中需要提供 `repair` 命令来修复已注册磁盘的身份信息。

## 一、修复 Fingerprint 匹配 Bug

**根因**: `macos.rs:126` 和 `linux.rs:119` 在运行时检测磁盘身份时调用 `generate_fingerprint(..., &Utc::now())`，而注册时的 fingerprint 用的是注册时刻的时间戳。两者永远不同，导致没有 serial/UUID 的磁盘始终 offline。

**修复方案**: 修改 `DiskIdentity::matches()` 的 fallback 分支，不再比较 fingerprint 字符串，改为直接比较 `volume_label + capacity_bytes`（这正是 fingerprint 编码的稳定信息）。检测器不再需要生成有意义的 fingerprint。

### 文件改动

1. **src/domain/disk.rs** — `DiskIdentity::matches()` 第 64-65 行
   - 将 `self.fingerprint == other.fingerprint` 替换为：
   ```rust
   self.volume_label.is_some()
       && self.volume_label == other.volume_label
       && self.capacity_bytes == other.capacity_bytes
   ```

2. **src/storage/platform/macos.rs** — `parse_diskutil_info()` 第 126 行
   - 将 `DiskIdentity::generate_fingerprint(volume_label.as_deref(), capacity_bytes, &chrono::Utc::now())` 改为 `String::new()`

3. **src/storage/platform/linux.rs** — `parse_lsblk_info()` 第 119 行
   - 同上，fingerprint 改为 `String::new()`

**向后兼容**: 已注册磁盘的 fingerprint 字段保留不变（仅作历史记录）。有 serial/UUID 的磁盘不受影响。只有 fingerprint-only 的磁盘会从"永远 offline"变为正确匹配。

## 二、添加 `update_disk_identity` 到 DiskRepo

**文件**: `src/persistence/disk_repo.rs`

新增方法，供 `repair` 命令使用：
```rust
pub fn update_disk_identity(&self, disk_id: &DiskId, identity: &DiskIdentity) -> Result<()>
```
更新 `serial`, `volume_uuid`, `volume_label`, `capacity_bytes`, `fingerprint` 五个字段。

## 三、重构 Handler 支持共享 AppContext

当前每个 handler 内部调用 `AppContext::init()`。交互模式需要复用同一个 AppContext。

**方案**: 为每个 handler 提取 `*_with_ctx` 版本，原函数保留为薄包装。

涉及文件（每个文件改动模式相同）：
- `src/cli/commands/disk.rs` → `handle_add_with_ctx(ctx, mount_point, name)` + `handle_list_with_ctx(ctx, detailed)`
- `src/cli/commands/scan.rs` → `handle_scan_with_ctx(ctx, all, disk, hash, full)`
- `src/cli/commands/search.rs` → `handle_search_with_ctx(ctx, keyword, min_size, max_size, ext, limit)`
- `src/cli/commands/get.rs` → `handle_get_with_ctx(ctx, entry_id, locate)`
- `src/cli/commands/store.rs` → `handle_store_with_ctx(ctx, paths, solid_layer, dedup, preview, yes)`
- `src/cli/commands/solid.rs` → `handle_set_with_ctx(ctx, path, disk)` + `handle_unset_with_ctx(ctx, path, disk)`
- `src/cli/commands/visualize.rs` → `handle_visualize_with_ctx(ctx, disk)`

每个文件的改动：将函数体移入 `*_with_ctx`，原函数变为 `let ctx = AppContext::init()?; xxx_with_ctx(&ctx, ...)`。

## 四、创建交互式 Shell

### 依赖

**Cargo.toml** 添加: `rustyline = "15"` — 提供 readline 编辑、历史记录、Ctrl-R 搜索。

### 新文件: `src/cli/interactive.rs`

**核心结构**:

```
pub fn run_interactive() -> Result<()>
  ├── 初始化 AppContext（整个会话复用）
  ├── 创建 rustyline::Editor（历史文件 ~/.disco/history.txt）
  ├── 打印欢迎信息 + 可用命令列表
  └── REPL 循环:
      ├── 读取输入 "disco> "
      ├── parse_shell_line() 分词（支持引号和转义）
      ├── dispatch() 路由到对应 handler
      └── 错误处理（打印错误，不退出循环）
```

**命令表**:

| 交互命令 | 映射 |
|---------|------|
| `disk add <mount> [--name N]` | `handle_add_with_ctx` |
| `disk list [-d]` | `handle_list_with_ctx` |
| `disk rename <id> <name>` | `disk_repo.update_disk_name` |
| `disk remove <id>` | `disk_repo.delete_disk`（需确认） |
| `scan [--all] [--disk D] [--hash]` | `handle_scan_with_ctx` |
| `search <keyword> [--ext E] [--limit N]` | `handle_search_with_ctx` |
| `get <id> [--locate]` | `handle_get_with_ctx` |
| `store <paths...> [--solid-layer S]` | `handle_store_with_ctx` |
| `solid set/unset <path> [--disk D]` | `handle_set/unset_with_ctx` |
| `visualize [--disk D]` | `handle_visualize_with_ctx` |
| `status` | 新功能：显示所有磁盘状态概览 |
| `repair` | 新功能：诊断并修复 offline 磁盘 |
| `help [command]` | 显示帮助 |
| `exit` / `quit` | 退出 |

### `status` 命令

调用 MountChecker 或复用 disk list 逻辑，显示：
- 每个磁盘的名称、ID、状态（Connected/Offline）、容量、文件数
- 汇总：总磁盘数、在线数、离线数、总索引文件数

### `repair` 命令

交互式修复流程：
1. 获取所有磁盘 + 当前挂载点
2. 对每个 offline 磁盘，扫描挂载点寻找 label 匹配的卷
3. 找到候选时，提示用户选择：
   ```
   磁盘 "素材盘-01" [abc123] 状态: OFFLINE
     发现挂载卷 /Volumes/素材盘-01 与此磁盘 label 匹配
     [1] 重新连接 — 更新身份信息以匹配当前卷
     [2] 跳过
     [3] 删除此磁盘注册
   ```
4. 选择"重新连接"时，调用 `disk_repo.update_disk_identity()` 更新 serial/uuid/fingerprint

## 五、修改 main.rs 入口

- `Cli.command` 改为 `Option<Commands>`
- 添加 `#[arg(short, long)] interactive: bool`
- dispatch 逻辑：`interactive` 或无子命令时进入 `run_interactive()`

## 六、模块注册

**src/cli/mod.rs** 添加 `pub mod interactive;`

## 实施顺序

1. 修复 fingerprint bug（domain/disk.rs, macos.rs, linux.rs）
2. 添加 `update_disk_identity` 到 disk_repo.rs
3. 重构所有 handler 提取 `*_with_ctx`
4. 添加 rustyline 依赖
5. 创建 `src/cli/interactive.rs`（REPL + status + repair）
6. 修改 main.rs 和 cli/mod.rs

## 验证

1. `cargo build` 编译通过
2. `cargo test --lib` 所有测试通过
3. `disco -i` 进入交互模式，测试：
   - `help` 显示命令列表
   - `disk list` 正常显示
   - `status` 显示磁盘概览
   - `repair` 能检测并修复 offline 磁盘
   - `exit` 正常退出
4. 原有 CLI 命令（`disco disk list` 等）不受影响
