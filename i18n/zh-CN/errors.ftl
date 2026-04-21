# Disco CLI 错误消息

# 硬盘错误
error-disk-not-registered = 硬盘 '{$disk}' 未在硬盘池中注册。
error-disk-not-connected = 硬盘 '{$disk}' 未连接到计算机。请先连接硬盘。
error-disk-identity-mismatch = 连接的硬盘与注册的硬盘不匹配。

# 条目/文件错误
error-entry-not-found = 在索引中找不到ID为 {$id} 的文件。
error-path-not-exist = 文件或文件夹 '{$path}' 不存在。
error-path-invalid = 路径 '{$path}' 无效。请检查拼写。

# 存储错误
error-file-too-large = 文件太大 ({$size} GB)，无法放入任何已连接的硬盘。
error-no-disks-connected = 没有连接硬盘。请至少连接一个硬盘到硬盘池。
error-file-exists = '{$path}' 处已存在文件。请选择其他位置或使用 --force 覆盖。

# 权限错误
error-permission-denied = 没有权限访问 '{$path}'。

# Solid错误
error-solid-cannot-split = 此文件夹标记为 'Solid'，不能跨多个硬盘拆分。

# 操作错误
error-operation-interrupted = {$operation} 操作被中断。
error-operation-failed = {$operation} 操作失败。
error-operation-cancelled = 操作已取消。

# 系统错误
error-database = 数据库错误: {$error}
error-filesystem = 文件系统错误: {$error}
error-config = 配置错误: {$error}
error-disk-detection = 无法检测硬盘信息: {$error}
error-database-upgrade = 数据库升级失败: {$error}

# 建议
suggest-add-disk = 使用 'disco disk add <挂载点>' 注册新硬盘。
suggest-connect-disk = 请连接硬盘后重试。
suggest-verify-disk = 验证是否连接了正确的硬盘，或使用 'disco repair' 更新硬盘身份。
suggest-check-path = 检查路径是否正确以及硬盘是否已挂载。
suggest-free-space = 释放已连接硬盘的空间或连接其他硬盘。
suggest-permissions = 检查文件权限或尝试以适当的权限运行。