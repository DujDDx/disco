# Disco CLI 菜单字符串

# 主菜单项
menu-disk-management = 磁盘管理
menu-scan-files = 扫描文件
menu-search-files = 搜索文件
menu-store-files = 存储文件
menu-retrieve-files = 检索文件
menu-view-status = 查看状态
menu-refresh-status = 刷新状态
menu-repair-offline = 修复离线
menu-visualize = 可视化
menu-settings = 设置
menu-exit = 退出菜单

# 子菜单 - 磁盘管理
submenu-disk-title = 磁盘管理
submenu-disk-add = 添加硬盘
submenu-disk-list = 列出硬盘
submenu-disk-add-prompt = 输入挂载点：

# 子菜单 - 扫描
submenu-scan-title = 扫描文件
submenu-scan-all = 扫描所有硬盘
submenu-scan-specific = 扫描指定硬盘
submenu-scan-select-disk = 选择硬盘

# 子菜单 - 设置
submenu-settings-title = 设置
submenu-settings-lang = 语言
submenu-settings-lang-desc = 按 Enter 或 左/右 键切换语言
submenu-settings-hash = 哈希校验
submenu-settings-hash-on = 开启
submenu-settings-hash-off = 关闭
submenu-settings-hash-desc = 开启后，扫描和存储时会计算文件哈希值用于校验

# 菜单帮助文本
menu-help-title = DISCO
menu-help-navigate = ↑/↓: 导航
menu-help-select = Enter: 选择
menu-help-quick = 1-9,0,q: 快速选择
menu-help-exit = Esc: 退出
menu-help-back = Backspace: 返回

# 菜单页脚
menu-returned = 已返回命令模式。
menu-back = 返回
menu-no-disks = 没有注册的硬盘。
menu-error = 错误：
menu-press-enter = 按 Enter 继续...

# 菜单描述（第二列）
menu-desc-disk = 添加、列出、重命名、删除硬盘
menu-desc-scan = 扫描硬盘文件
menu-desc-search = 搜索索引文件
menu-desc-store = 存储文件到硬盘
menu-desc-retrieve = 从硬盘检索文件
menu-desc-status = 显示硬盘状态概览
menu-desc-refresh = 强制刷新挂载检测
menu-desc-repair = 修复离线硬盘身份
menu-desc-visualize = 打开TUI可视化界面
menu-desc-settings = 配置哈希校验
menu-desc-exit = 返回命令模式

# 使用提示
usage-disk = 用法: disk <add|list|rename|remove>
usage-disk-add = 用法: disk add <挂载点> [--name N]
usage-disk-rename = 用法: disk rename <硬盘ID> <新名称>
usage-disk-remove = 用法: disk remove <硬盘ID>
usage-scan = 用法: scan [--all] [--disk D] [--hash] [--full]
usage-search = 用法: search <关键词> [--ext E] [--limit N]
usage-get = 用法: get <条目ID> [--locate]
usage-store = 用法: store <路径...> [--solid-layer S]
usage-retrieve = 用法: retrieve <关键词>
usage-solid = 用法: solid <set|unset> <路径> [--disk D]

# 未知命令
unknown-command = 未知命令: {$command}
unknown-disk-subcommand = 未知的硬盘子命令: {$command}
unknown-solid-subcommand = 未知的solid子命令: {$command}
available-disk-commands = 可用: add, list, rename, remove

# 提示
prompt-enter-mount-point = 输入挂载点：
prompt-enter-keyword = 输入关键词：
prompt-enter-paths = 输入文件路径（多个用逗号分隔）：
prompt-enter-solid-layer = 输入SolidLayer深度（默认0不分割）：
prompt-enter-destination = 输入目标目录 [默认: ./]：
prompt-enter-disk-id = 输入硬盘ID：
prompt-select-files = 输入要检索的文件编号（如 1,3,5）或 'all'：