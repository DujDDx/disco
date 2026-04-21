# Error messages for Disco CLI

# Disk errors
error-disk-not-registered = The disk '{$disk}' is not registered in your disk pool.
error-disk-not-connected = The disk '{$disk}' is not connected to your computer. Please connect it first.
error-disk-identity-mismatch = The connected disk appears to be different from the registered one.

# Entry/File errors
error-entry-not-found = Could not find the file with ID {$id} in the index.
error-path-not-exist = The file or folder '{$path}' does not exist.
error-path-invalid = The path '{$path}' is not valid. Please check the spelling.

# Storage errors
error-file-too-large = The file is too large ({$size} GB) to fit on any of your connected disks.
error-no-disks-connected = No disks are connected. Please connect at least one disk to your disk pool.
error-file-exists = A file already exists at '{$path}'. Choose a different location or use --force to overwrite.

# Permission errors
error-permission-denied = You don't have permission to access '{$path}'.

# Solid errors
error-solid-cannot-split = This folder is marked as 'Solid' and cannot be split across multiple disks.

# Operation errors
error-operation-interrupted = The {$operation} operation was interrupted.
error-operation-failed = The {$operation} operation failed.
error-operation-cancelled = The operation was cancelled.

# System errors
error-database = A database error occurred: {$error}
error-filesystem = A file system error occurred: {$error}
error-config = A configuration error occurred: {$error}
error-disk-detection = Could not detect disk information: {$error}
error-database-upgrade = Database upgrade failed: {$error}

# Suggestions
suggest-add-disk = Use 'disco disk add <mount-point>' to register a new disk.
suggest-connect-disk = Please connect the disk and try again.
suggest-verify-disk = Verify that the correct disk is connected, or use 'disco repair' to update the disk identity.
suggest-check-path = Check that the path is correct and the disk is mounted.
suggest-free-space = Free up space on connected disks or connect additional disks.
suggest-permissions = Check file permissions or try running with appropriate privileges.