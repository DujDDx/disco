# Disco MVP 构建计划

## Context

Disco 是一个本地多硬盘存储调度 CLI 工具，用于将多块独立硬盘组织为"硬盘池"，提供离线索引检索、智能存储调度、Solid/SolidLayer 规则和终端可视化能力。项目从零开始构建，目标是先交付 MVP（第一阶段）。

技术选型：Rust + SQLite，目标平台 macOS + Linux。

## 项目结构

单 crate + 模块化分层，MVP 阶段不需要 workspace 的编译隔离开销，后续可拆分。

```
Disco/
├── Cargo.toml
├── docs/requirements.md
├── src/
│   ├── main.rs                     # 入口，clap CLI 定义
│   ├── lib.rs                      # 模块树 + 共享 Error 类型
│   ├── cli/                        # CLI 层
│   │   ├── mod.rs
│   │   ├── commands/               # 各子命令实现
│   │   │   ├── mod.rs
│   │   │   ├── disk.rs             # disk add / list
│   │   │   ├── scan.rs             # scan --all / --disk
│   │   │   ├── search.rs           # search <keyword>
│   │   │   ├── get.rs              # get <entry-id>
│   │   │   ├── store.rs            # store <path...> --solid-layer=N
│   │   │   ├── solid.rs            # solid set / unset
│   │   │   └── visualize.rs        # 基础树形视图
│   │   └── display.rs              # 终端输出辅助（表格、进度条）
│   ├── domain/                     # 核心领域模型（纯数据，无 IO）
│   │   ├── mod.rs
│   │   ├── disk.rs                 # Disk, DiskId, DiskIdentity, MountStatus
│   │   ├── entry.rs                # IndexEntry, EntryType, EntryStatus
│   │   ├── solid.rs                # SolidLayerDepth, AtomicUnit, 切分逻辑
│   │   └── plan.rs                 # StorePlan, PlanItem
│   ├── index/                      # 索引服务
│   │   ├── mod.rs
│   │   ├── scanner.rs              # 全量扫描，目录遍历
│   │   ├── hasher.rs               # BLAKE3 流式哈希
│   │   └── query.rs                # 搜索/过滤
│   ├── planner/                    # 调度规划
│   │   ├── mod.rs
│   │   ├── splitter.rs             # 原子单元切分
│   │   ├── strategy.rs             # DiskSelectionStrategy trait + BestFit
│   │   └── store_planner.rs        # 编排切分+策略→StorePlan
│   ├── executor/                   # 执行服务
│   │   ├── mod.rs
│   │   ├── copy.rs                 # 文件/目录复制 + 进度
│   │   ├── verify.rs               # 复制后哈希校验
│   │   └── task.rs                 # 任务状态机，中断恢复
│   ├── storage/                    # 存储适配层
│   │   ├── mod.rs
│   │   ├── platform/
│   │   │   ├── mod.rs              # PlatformDiskDetector trait + 平台选择
│   │   │   ├── macos.rs            # diskutil 实现
│   │   │   └── linux.rs            # lsblk/blkid 实现
│   │   ├── fs.rs                   # 文件系统操作抽象
│   │   └── mount.rs                # 挂载检测，硬盘匹配
│   └── persistence/                # 持久化层
│       ├── mod.rs
│       ├── db.rs                   # SQLite 连接 + 迁移
│       ├── schema.rs               # 迁移 SQL 定义
│       ├── disk_repo.rs            # disks 表 CRUD
│       ├── entry_repo.rs           # entries 表 CRUD
│       ├── task_repo.rs            # tasks 表 CRUD
│       └── config.rs               # 配置读写 + 数据目录解析
└── tests/                          # 集成测试
    ├── common/mod.rs
    ├── scan_test.rs
    ├── store_test.rs
    ├── search_test.rs
    └── solid_test.rs
```

## 核心依赖

| 用途 | Crate |
|------|-------|
| CLI 解析 | clap 4 (derive) |
| SQLite | rusqlite (bundled) + rusqlite_migration |
| TUI | ratatui + crossterm |
| 进度条 | indicatif |
| 文件遍历 | walkdir |
| 文件哈希 | blake3 |
| 模糊搜索 | nucleo-matcher |
| 磁盘信息 | sysinfo + 平台命令 (diskutil / lsblk) |
| 序列化 | serde + serde_json |
| 数据目录 | directories |
| 错误处理 | anyhow + thiserror |
| 日志 | tracing + tracing-subscriber |
| 时间 | chrono |
| 测试 | tempfile, assert_cmd, predicates |

## SQLite Schema

```sql
CREATE TABLE disks (
    disk_id          TEXT PRIMARY KEY,
    name             TEXT NOT NULL,
    serial           TEXT,
    volume_uuid      TEXT,
    volume_label     TEXT,
    capacity_bytes   INTEGER NOT NULL,
    fingerprint      TEXT NOT NULL,
    first_registered TEXT NOT NULL,
    last_mount_point TEXT
);

CREATE TABLE entries (
    entry_id              INTEGER PRIMARY KEY AUTOINCREMENT,
    disk_id               TEXT NOT NULL REFERENCES disks(disk_id),
    relative_path         TEXT NOT NULL,
    file_name             TEXT NOT NULL,
    size                  INTEGER NOT NULL,
    hash                  TEXT,
    mtime                 TEXT NOT NULL,
    entry_type            TEXT NOT NULL CHECK(entry_type IN ('file', 'dir')),
    solid_flag            INTEGER NOT NULL DEFAULT 0,
    last_seen_mount_point TEXT NOT NULL,
    indexed_at            TEXT NOT NULL,
    status                TEXT NOT NULL DEFAULT 'normal'
                          CHECK(status IN ('normal', 'missing', 'pending_confirm')),
    UNIQUE(disk_id, relative_path)
);

CREATE INDEX idx_entries_file_name ON entries(file_name);
CREATE INDEX idx_entries_disk_id ON entries(disk_id);
CREATE INDEX idx_entries_hash ON entries(hash) WHERE hash IS NOT NULL;
CREATE INDEX idx_entries_file_name_lower ON entries(lower(file_name));

CREATE TABLE tasks (
    task_id    TEXT PRIMARY KEY,
    task_type  TEXT NOT NULL CHECK(task_type IN ('store', 'scan')),
    status     TEXT NOT NULL CHECK(status IN ('pending', 'running', 'completed', 'failed', 'interrupted')),
    payload    TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

## 关键算法

### 原子单元切分 (SolidLayer)

```
split(root, solid_layer, solid_checker):
  文件 → 直接返回 [AtomicUnit(root)]
  solid_layer=0 → 返回 [AtomicUnit(root整体)]
  目录 → 遍历子项:
    - 子项是文件 → AtomicUnit(子文件)
    - 子项被标记 Solid → AtomicUnit(子目录整体)，不再递归
    - solid_layer=1 → AtomicUnit(子目录整体)
    - 否则 → 递归 split(子目录, solid_layer-1, solid_checker)
```

### 磁盘选择 (Best Fit Decreasing)

1. 按大小降序排列所有原子单元（大的先分配）
2. 对每个单元，筛选剩余空间 >= 单元大小的候选盘
3. 在候选盘中选"写入后剩余空间最小"的（Best Fit）
4. 无候选盘 → 报错，不执行部分写入

### 搜索 (两阶段)

1. SQL 预过滤：`WHERE lower(file_name) LIKE '%keyword%' LIMIT 500`
2. 内存模糊排序：nucleo-matcher 对结果重新评分排序

### 硬盘身份匹配优先级

serial > volume_uuid > fingerprint (label+capacity+注册时间哈希)

## 实现步骤（按依赖顺序）

### Phase A: 基础层（无相互依赖，可并行）

**Step 1: 项目骨架 + Cargo.toml + clap CLI**
- 创建 Cargo.toml，所有依赖
- main.rs 用 clap derive 定义所有 MVP 命令（stub 实现）
- lib.rs 模块树，所有 mod.rs 空文件
- 验证：`cargo build` 成功，`disco --help` 输出所有命令

**Step 2: 领域模型 (domain/)**
- Disk, DiskId, DiskIdentity, MountStatus
- IndexEntry, EntryType, EntryStatus
- SolidLayerDepth, AtomicUnit
- StorePlan, PlanItem
- 单元测试：SolidLayerDepth 解析

**Step 3: 持久化基础 (persistence/db.rs + schema.rs)**
- Database::open() / open_in_memory()
- 迁移定义和执行
- Database::transaction()
- 测试：内存 DB 创建所有表

### Phase B: 仓库层（依赖 A）

**Step 4: disk_repo.rs**
- insert_disk, get_disk_by_id, list_disks, update_last_mount_point, find_disk_by_identity
- 集成测试

**Step 5: entry_repo.rs**
- upsert_entry, batch_upsert, search_by_name, get_entry_by_id, get_entries_by_disk
- mark_missing, set_solid_flag, unset_solid_flag
- 集成测试

**Step 6: task_repo.rs + config.rs**
- 任务 CRUD + 配置 KV 存储
- DataDir 路径解析 (~/.disco/)
- 集成测试

### Phase C: 平台适配 + 服务层（依赖 A+B）

**Step 7: 平台磁盘检测 (storage/platform/)**
- PlatformDiskDetector trait
- macOS: diskutil 解析
- Linux: lsblk + blkid 解析
- 编译时 cfg 选择平台实现

**Step 8: 文件系统适配 (storage/fs.rs)**
- FsAdapter trait: walk_directory, copy_file, copy_dir_recursive, file_size, dir_total_size
- tempfile 单元测试

**Step 9: 挂载检测 (storage/mount.rs)**
- check_disk_status, find_mount_point
- 身份匹配逻辑（serial > uuid > fingerprint）

**Step 10: BLAKE3 哈希 (index/hasher.rs)**
- hash_file: 64KB 流式读取
- hash_files_in_dir
- 单元测试

**Step 11: 扫描器 (index/scanner.rs)**
- full_scan: walkdir 遍历 → 批量 upsert → 标记缺失
- indicatif 进度条
- 任务记录用于中断恢复
- 集成测试：tempfile 目录树扫描

**Step 12: 搜索引擎 (index/query.rs)**
- search: SQL LIKE 预过滤 + nucleo 模糊排序
- 按大小/扩展名过滤（可选）
- 集成测试

**Step 13: 原子单元切分 (planner/splitter.rs)**
- split_into_atomic_units 实现
- 单元测试：各种 SolidLayer + Solid 组合

**Step 14: 磁盘选择策略 (planner/strategy.rs)**
- DiskSelectionStrategy trait
- BestFitStrategy 实现
- 单元测试：多种空间分配场景

**Step 15: 存储规划器 (planner/store_planner.rs)**
- 编排 splitter + strategy → StorePlan
- 验证：单元过大无盘可放时报错

**Step 16: 执行器 (executor/)**
- copy.rs: 文件/目录复制 + 进度回调
- verify.rs: 复制后哈希校验
- task.rs: 任务状态机（pending→running→completed/failed/interrupted）

### Phase D: CLI 命令接入（依赖 C）

**Step 17: `disco disk add` + `disco disk list`**
- add: 检测磁盘身份 → 提示输入名称 → 入库
- list: 查询所有盘 + 当前挂载状态 → 格式化表格输出

**Step 18: `disco scan --all` + `disco scan --disk`**
- 调用 scanner，显示进度条和扫描报告

**Step 19: `disco search` + `disco get`**
- search: 模糊搜索 → 格式化结果（文件名、硬盘名、路径、挂载状态）
- get: 定位文件 → 检查挂载 → 提示连接或输出路径

**Step 20: `disco store`**
- 解析拖入路径（兼容空格/转义）
- 切分原子单元 → 生成调度计划 → 预览 → 确认 → 执行复制 → 更新索引
- 支持 --solid-layer 参数

**Step 21: `disco solid set/unset`**
- 设置/取消索引中目录的 Solid 标记

**Step 22: `disco visualize` — 基础树形视图**
- ratatui 树形展示：硬盘池 → 硬盘 → 目录层级
- 显示大小、挂载状态、Solid 标记
- 键盘导航：折叠/展开、按硬盘过滤

### Phase E: 收尾

**Step 23: 集成测试**
- 端到端：注册盘 → 扫描 → 搜索 → 存储 → 验证索引
- Solid/SolidLayer 组合测试
- 中断恢复测试

**Step 24: 错误处理 + 用户体验打磨**
- 所有用户可见错误信息可读化
- 高风险操作二次确认
- 路径格式错误提示

## 验证方式

1. `cargo build` — 编译通过
2. `cargo test` — 所有单元测试和集成测试通过
3. `cargo clippy` — 无警告
4. 手动测试流程：
   - `disco disk add /Volumes/TestDisk` → 注册成功
   - `disco disk list` → 显示已注册硬盘及状态
   - `disco scan --all` → 扫描完成，显示进度和报告
   - `disco search test` → 返回匹配文件列表
   - `disco store ./some-folder --solid-layer=1` → 预览计划 → 确认 → 复制完成
   - `disco solid set /path/to/dir` → 标记成功
   - `disco visualize` → 树形视图可交互