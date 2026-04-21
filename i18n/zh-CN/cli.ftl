# Disco CLI 命令行消息

# 交互终端
shell-welcome-title = Disco 交互终端
shell-welcome-help = 输入 'help' 查看可用命令，'exit' 退出。
shell-welcome-menu = 使用 'menu' 进入方向键导航的可视化菜单。
shell-prompt = disco>
shell-input-error = 输入错误: {$error}
shell-interrupted = ^C
shell-eof = ^D

# 帮助文本
help-available-commands = 可用命令
help-detailed = 显示详细帮助
help-exit = 退出终端
help-general = 输入 'help' 查看通用帮助。
help-no-detail = 无详细帮助: {$command}
help-disk-commands = 硬盘命令
help-disk-add = 注册新硬盘
help-disk-list = 列出已注册硬盘
help-disk-rename = 重命名硬盘
help-disk-remove = 删除硬盘
help-disk-add-desc = 在指定挂载点注册新硬盘。
help-disk-list-desc = 列出所有已注册硬盘，可选详细信息。
help-disk-rename-desc = 更改已注册硬盘的名称。
help-disk-remove-desc = 删除硬盘及其索引条目（需确认）。
help-scan-commands = 扫描命令
help-scan = 扫描硬盘文件
help-scan-all-desc = 扫描所有已注册硬盘
help-scan-disk-desc = 按ID或名称扫描指定硬盘
help-scan-hash-desc = 扫描时计算文件哈希
help-scan-full-desc = 强制完全扫描（非增量）
help-search = 搜索索引文件
help-get = 获取文件信息和位置
help-store = 存储文件到硬盘
help-retrieve = 从硬盘检索文件
help-solid-set = 标记目录为Solid
help-solid-unset = 移除Solid标记
help-visualize = 打开可视化界面
help-status = 显示硬盘状态概览
help-status-commands = 状态命令
help-status-desc = 显示所有硬盘概览：
help-status-detail1 = 硬盘名称、ID和挂载状态
help-status-detail2 = 容量和索引文件数
help-status-detail3 = 汇总统计
help-refresh = 刷新硬盘挂载状态
help-refresh-commands = 刷新命令
help-refresh-desc = 强制刷新所有硬盘的挂载状态。
help-refresh-desc2 = 显示离线硬盘的详细诊断信息。
help-repair = 修复离线硬盘身份
help-repair-commands = 修复命令
help-repair-desc = 交互式修复离线硬盘。
help-repair-desc2 = 检测因身份不匹配而显示离线的硬盘，
help-repair-desc3 = 提供重新连接、跳过或删除的选项。
help-menu = 打开可视化菜单导航

# 硬盘命令
disk-detecting = 正在检测硬盘 {$path}...
disk-already-registered = 硬盘已注册为: {$name}
disk-id = ID: {$id}
disk-default-name = 新硬盘
disk-name-prompt = 输入硬盘名称 [{$default}]:
disk-registered-success = 硬盘注册成功！
disk-renamed-success = 硬盘已重命名为: {$name}
disk-removed-success = 硬盘已删除。
disk-name = 名称: {$name}
disk-capacity = 容量: {$size}

disk-list-title = 已注册硬盘 ({$count})
disk-status = 状态: {$status}
disk-mount-point = 挂载点: {$path}
disk-last-mount = 上次挂载: {$path}
disk-serial = 序列号: {$serial}
disk-uuid = 卷UUID: {$uuid}
disk-label = 卷标: {$label}
disk-registered = 注册时间: {$date}

# 扫描命令
scan-scanning = 正在扫描硬盘: {$name} [{$id}]
scan-not-mounted = 硬盘未挂载，跳过...
scan-mount-point = 挂载点: {$path}
scan-hash-enabled = 哈希计算: 已启用
scan-complete = 扫描完成！
scan-results = 扫描结果：
scan-files-added = 新增文件: {$count}
scan-files-updated = 更新文件: {$count}
scan-dirs-added = 新增目录: {$count}
scan-dirs-updated = 更新目录: {$count}
scan-files-missing = 标记丢失文件: {$count}
scan-errors = 错误: {$count}
scan-total-files = 总文件数: {$count}
scan-total-dirs = 总目录数: {$count}

# 搜索命令
search-no-results = 没有找到匹配 '{$keyword}' 的文件
search-results-title = '{$keyword}' 的搜索结果 (找到 {$count} 个)
search-use-get = 使用 'disco get <ID>' 定位特定文件。

# 获取命令
get-invalid-id = 无效的条目ID
get-file-info = 文件信息：
get-name = 名称: {$name}
get-size = 大小: {$size}
get-disk = 硬盘: {$name} [{$id}]
get-path = 路径: {$path}
get-hash = 哈希: {$hash}
get-mounted-at = 硬盘挂载于: {$path}
get-full-path = 完整路径: {$path}
get-verified = 文件已验证
get-not-found = 警告: 在预期位置未找到文件
get-disk-not-mounted = 硬盘 '{$name}' 当前未挂载。
get-last-mount = 上次已知挂载点: {$path}
get-please-connect = 请连接硬盘以访问此文件。

# 存储命令
store-solid-layer = SolidLayer: {$depth}
store-path-not-found = 路径不存在，跳过: {$path}
store-input-paths = 输入路径：
store-no-disks = 当前没有挂载的硬盘。
store-connect-disk = 请至少连接一个硬盘到存储池。
store-available-disks = 可用硬盘：
store-disk-free = {$name} [{$id}]: {$size} 可用
store-atomic-units = 原子单元 ({$count})
store-unit-info = {$name} ({$size}, {$files} 个文件)
store-plan-title = 存储计划：
store-plan-item = {$path} → {$disk} [{$size}]
store-total = 总计: {$files} 个文件, {$size}
store-preview-mode = 预览模式 - 未复制任何文件。
store-proceed = 是否执行存储？[y/N]
store-copying = 正在复制文件...
store-copied-success = 复制成功
store-copied-fail = 失败: {$error}
store-stored = 已存储 {$files} 个文件 ({$size})
store-failed-items = {$count} 个项目存储失败
store-indexing = 正在更新索引...
store-indexing-folder = 正在索引 {$name}...
store-indexed = 已索引 {$count} 条记录

# 检索命令
retrieve-searching = 正在搜索: {$keyword}
retrieve-no-results = 没有找到匹配的文件或文件夹。
retrieve-results-title = 搜索结果：
retrieve-folders-title = 文件夹（跨硬盘聚合）：
retrieve-files-title = 文件：
retrieve-total = 总计: {$folders} 个文件夹, {$files} 个文件
retrieve-retrieving = 正在检索文件...
retrieve-retrieving-folder = 正在检索文件夹: {$name}
retrieve-found-files = 在 {$disks} 个硬盘上找到 {$files} 个文件
retrieve-retrieved = 已检索 {$files} 个文件 ({$size})
retrieve-failed-files = {$count} 个文件检索失败
retrieve-copying = 正在复制 {$name}...
retrieve-saved = 已保存到 {$path}

# 可视化命令
viz-title-disk-list = 硬盘列表
viz-title-folder = 文件夹: {$path}
viz-title-folder-tree = 文件夹树（根目录）
viz-title-usage = 空间占用（根目录）
viz-title-usage-path = 空间占用: {$path}
viz-disk-title = 硬盘（基于本地索引 | Enter: 文件夹 | U: 占用视图）
viz-folder-title = {$path} (Enter: 进入 | Backspace: 返回 | {$count}项)
viz-usage-title = {$path} ({$count}项)
viz-help-disk = ↑↓: 导航 │ Enter: 文件夹 │ U: 占用视图 │ Q: 退出
viz-help-tree = ↑↓: 导航 │ Enter: 进入文件夹 │ Backspace/Esc: 返回 │ Q: 退出
viz-help-usage = ↑↓: 选择 │ ←→: 切换硬盘 │ Enter: 进入 │ Backspace: 返回 │ Q: 退出

# 状态命令
status-title = 硬盘状态概览
status-summary = 摘要
status-online-count = {$count} 个在线
status-offline-count = {$count} 个离线
status-total-files = 总索引文件

# 刷新命令
refresh-title = 正在刷新硬盘状态...
refresh-mount-title = 检测到的挂载点
refresh-disk-title = 硬盘状态结果
refresh-diagnostic = 诊断
refresh-no-match = 未找到匹配的挂载点
refresh-potential = 可能的匹配

# 修复命令
repair-all-online = 所有硬盘都在线。无需修复。
repair-found-offline = 发现 {$count} 个离线硬盘：
repair-disk-label = 硬盘
repair-volume-label = 卷标
repair-no-candidates = 未找到匹配的挂载点。
repair-skip = 跳过
repair-remove = 删除此硬盘注册
repair-select = 选择操作：
repair-candidates = 发现 {$count} 个候选挂载点：
repair-reconnect = 重新连接 - 更新身份以匹配当前卷
repair-skip-disk = 跳过此硬盘
repair-delete = 删除此硬盘注册
repair-identity-updated = 硬盘身份已更新并重新连接。
repair-new-mount = 新挂载点
repair-removed = 硬盘已删除。
repair-skipped = 已跳过。
repair-complete = 修复完成。

# Solid命令
solid-set = Solid标记已设置: {$path}
solid-set-desc = 此目录在存储操作期间不会被拆分。
solid-unset = Solid标记已移除: {$path}

# 配置命令
config-current-lang = 当前语言
config-available-langs = 可用语言
config-lang-set = 语言已设置为: {$lang}
config-usage = 用法

# 索引
index-updated = 运行 'disco scan --all' 更新索引中的新文件。