# Disco CLI 帮助文本和命令描述

# 主命令描述
cmd-disk-desc = 管理硬盘池
cmd-scan-desc = 扫描硬盘并构建/更新文件索引
cmd-search-desc = 在索引中搜索文件
cmd-get-desc = 通过条目ID获取文件位置
cmd-store-desc = 将文件/文件夹存储到硬盘池
cmd-retrieve-desc = 从硬盘池检索文件
cmd-solid-desc = 管理目录上的Solid标记
cmd-visualize-desc = 打开终端可视化界面
cmd-menu-desc = 打开方向键导航的可视化菜单

# 硬盘子命令
cmd-disk-add-desc = 添加新硬盘到硬盘池
cmd-disk-list-desc = 列出所有已注册的硬盘及其状态

# Solid子命令
cmd-solid-set-desc = 在目录上设置Solid标记
cmd-solid-unset-desc = 移除目录上的Solid标记

# 参数描述
arg-verbose = 增加日志详细程度 (-v, -vv, -vvv)
arg-interactive = 启动交互终端模式

# 硬盘命令参数
arg-disk-mount = 要添加的硬盘挂载点（如 macOS 上的 /Volumes/MyDisk）
arg-disk-name = 硬盘自定义名称（如未提供将提示输入）
arg-disk-detailed = 显示详细信息包括身份详情

# 扫描命令参数
arg-scan-all = 扫描所有已注册的硬盘
arg-scan-disk = 通过ID或名称扫描指定硬盘
arg-scan-hash = 在扫描时启用哈希计算
arg-scan-full = 强制完全扫描而非增量扫描

# 搜索命令参数
arg-search-keyword = 搜索关键词（匹配文件名）
arg-search-min-size = 按最小文件大小过滤（字节）
arg-search-max-size = 按最大文件大小过滤（字节）
arg-search-ext = 按文件扩展名过滤（如 .pdf）
arg-search-limit = 限制结果数量

# 获取命令参数
arg-get-id = 搜索结果中的条目ID
arg-get-locate = 检查硬盘是否挂载并提供访问路径

# 存储命令参数
arg-store-paths = 要存储的路径（支持拖放路径）
arg-store-solid-layer = SolidLayer深度（0=不分割，1=分割到第一级，inf=分割到文件）
arg-store-dedup = 启用基于哈希的去重
arg-store-preview = 预览存储计划而不执行
arg-store-yes = 跳过确认提示

# 检索命令参数
arg-retrieve-keyword = 搜索文件的关键词
arg-retrieve-dest = 检索文件的目标目录
arg-retrieve-ext = 文件扩展名过滤
arg-retrieve-limit = 最大结果显示数量
arg-retrieve-all = 检索所有匹配文件无需确认
arg-retrieve-folder = 搜索文件夹而非文件

# Solid命令参数
arg-solid-path = 要标记的目录索引路径
arg-solid-disk = 硬盘ID或名称（路径不明确时需要）

# 可视化命令参数
arg-viz-usage = 以硬盘使用视图（树状图）启动
arg-viz-tree = 以树状视图启动
arg-viz-disk = 过滤到指定硬盘