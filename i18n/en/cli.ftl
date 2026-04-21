# CLI messages for Disco CLI

# Interactive shell
shell-welcome-title = Disco Interactive Shell
shell-welcome-help = Type 'help' for available commands, 'exit' to quit.
shell-welcome-menu = Use 'menu' for visual navigation with arrow keys.
shell-prompt = disco>
shell-input-error = Input error: {$error}
shell-interrupted = ^C
shell-eof = ^D

# Help text
help-available-commands = Available Commands
help-detailed = Show detailed help
help-exit = Exit the shell
help-general = Type 'help' for general help.
help-no-detail = No detailed help for: {$command}
help-disk-commands = Disk Commands
help-disk-add = Register a new disk
help-disk-list = List registered disks
help-disk-rename = Rename a disk
help-disk-remove = Remove a disk
help-disk-add-desc = Register a new disk at the specified mount point.
help-disk-list-desc = List all registered disks with optional details.
help-disk-rename-desc = Change the name of a registered disk.
help-disk-remove-desc = Remove a disk and its indexed entries (requires confirmation).
help-scan-commands = Scan Command
help-scan = Scan disks for files
help-scan-all-desc = Scan all registered disks
help-scan-disk-desc = Scan specific disk by ID or name
help-scan-hash-desc = Calculate file hashes during scan
help-scan-full-desc = Force full scan (not incremental)
help-search = Search indexed files
help-get = Get file info and location
help-store = Store files to disks
help-retrieve = Retrieve files from disks
help-solid-set = Mark directory as solid
help-solid-unset = Remove solid marker
help-visualize = Open visualization UI
help-status = Show disk status overview
help-status-commands = Status Command
help-status-desc = Display overview of all disks:
help-status-detail1 = Disk name, ID, and mount status
help-status-detail2 = Capacity and indexed file count
help-status-detail3 = Summary totals
help-refresh = Refresh disk mount status
help-refresh-commands = Refresh Command
help-refresh-desc = Force refresh mount status for all disks.
help-refresh-desc2 = Shows detailed diagnostics for offline disks.
help-repair = Repair offline disk identities
help-repair-commands = Repair Command
help-repair-desc = Interactive repair for offline disks.
help-repair-desc2 = Detects disks that appear offline due to identity mismatch,
help-repair-desc3 = and offers options to reconnect, skip, or remove.
help-menu = Open visual menu navigation

# Disk commands
disk-detecting = Detecting disk at {$path}...
disk-already-registered = Disk already registered as: {$name}
disk-id = ID: {$id}
disk-default-name = New Disk
disk-name-prompt = Enter disk name [{$default}]:
disk-registered-success = Disk registered successfully!
disk-renamed-success = Disk renamed to: {$name}
disk-removed-success = Disk removed.
disk-name = Name: {$name}
disk-capacity = Capacity: {$size}

disk-list-title = Registered Disks ({$count})
disk-status = Status: {$status}
disk-mount-point = Mount: {$path}
disk-last-mount = Last mount: {$path}
disk-serial = Serial: {$serial}
disk-uuid = Volume UUID: {$uuid}
disk-label = Volume Label: {$label}
disk-registered = Registered: {$date}

# Scan commands
scan-scanning = Scanning disk: {$name} [{$id}]
scan-not-mounted = Disk not mounted, skipping...
scan-mount-point = Mount point: {$path}
scan-hash-enabled = Hash calculation: enabled
scan-complete = Scan complete!
scan-results = Scan completed:
scan-files-added = Files added: {$count}
scan-files-updated = Files updated: {$count}
scan-dirs-added = Dirs added: {$count}
scan-dirs-updated = Dirs updated: {$count}
scan-files-missing = Files marked missing: {$count}
scan-errors = Errors: {$count}
scan-total-files = Total files indexed: {$count}
scan-total-dirs = Total directories indexed: {$count}

# Search commands
search-no-results = No files found matching '{$keyword}'
search-results-title = Search results for '{$keyword}' ({$count} found)
search-use-get = Use 'disco get <ID>' to locate a specific file.

# Get commands
get-invalid-id = Invalid entry ID
get-file-info = File Information:
get-name = Name: {$name}
get-size = Size: {$size}
get-disk = Disk: {$name} [{$id}]
get-path = Path: {$path}
get-hash = Hash: {$hash}
get-mounted-at = Disk is mounted at: {$path}
get-full-path = Full path: {$path}
get-verified = File verified
get-not-found = Warning: File not found at expected location
get-disk-not-mounted = Disk '{$name}' is not currently mounted.
get-last-mount = Last known mount point: {$path}
get-please-connect = Please connect the disk to access this file.

# Store commands
store-solid-layer = SolidLayer: {$depth}
store-path-not-found = Path not found, skipping: {$path}
store-input-paths = Input paths:
store-no-disks = No disks are currently mounted.
store-connect-disk = Please connect at least one disk to the pool.
store-available-disks = Available disks:
store-disk-free = {$name} [{$id}]: {$size} free
store-atomic-units = Atomic units ({$count})
store-unit-info = {$name} ({$size}, {$files} files)
store-plan-title = Storage Plan:
store-plan-item = {$path} → {$disk} [{$size}]
store-total = Total: {$files} files, {$size}
store-preview-mode = Preview mode - no files were copied.
store-proceed = Proceed with storage? [y/N]
store-copying = Copying Files...
store-copied-success = Copied successfully
store-copied-fail = Failed: {$error}
store-stored = Stored {$files} files ({$size})
store-failed-items = Failed to store {$count} items
store-indexing = Updating Index...
store-indexing-folder = Indexing {$name}...
store-indexed = Indexed {$count} entries

# Retrieve commands
retrieve-searching = Searching for: {$keyword}
retrieve-no-results = No files or folders found matching the keyword.
retrieve-results-title = Search Results:
retrieve-folders-title = Folders (aggregated across disks):
retrieve-files-title = Files:
retrieve-total = Total: {$folders} folders, {$files} files
retrieve-retrieving = Retrieving Files...
retrieve-retrieving-folder = Retrieving folder: {$name}
retrieve-found-files = Found {$files} files across {$disks} disk(s)
retrieve-retrieved = Retrieved {$files} files ({$size})
retrieve-failed-files = Failed to retrieve {$count} files
retrieve-copying = Copying {$name}...
retrieve-saved = Saved to {$path}

# Visualize commands
viz-title-disk-list = Disk List
viz-title-folder = Folder: {$path}
viz-title-folder-tree = Folder Tree (Root)
viz-title-usage = Usage View (Root)
viz-title-usage-path = Usage: {$path}
viz-disk-title = Disks (Based on local index | Enter: Folders | U: Usage)
viz-folder-title = {$path} (Enter: Open | Backspace: Back | {$count} items)
viz-usage-title = {$path} ({$count} items)
viz-help-disk = ↑↓: Navigate │ Enter: Folders │ U: Usage │ Q: Quit
viz-help-tree = ↑↓: Navigate │ Enter: Open │ Backspace/Esc: Back │ Q: Quit
viz-help-usage = ↑↓: Select │ ←→: Change Disk │ Enter: Open │ Backspace: Back │ Q: Quit

# Status commands
status-title = Disk Status Overview
status-summary = Summary
status-online-count = {$count} online
status-offline-count = {$count} offline
status-total-files = Total indexed files

# Refresh commands
refresh-title = Refreshing Disk Status...
refresh-mount-title = Mount Points Detected
refresh-disk-title = Disk Status Results
refresh-diagnostic = Diagnostic
refresh-no-match = No matching mount found
refresh-potential = Potential matches

# Repair commands
repair-all-online = All disks are online. No repair needed.
repair-found-offline = Found {$count} offline disk(s):
repair-disk-label = Disk
repair-volume-label = Volume label
repair-no-candidates = No matching mount points found.
repair-skip = Skip
repair-remove = Remove this disk registration
repair-select = Select option:
repair-candidates = Found {$count} candidate mount point(s):
repair-reconnect = Reconnect - update identity to match current volume
repair-skip-disk = Skip this disk
repair-delete = Delete this disk registration
repair-identity-updated = Disk identity updated and reconnected.
repair-new-mount = New mount point
repair-removed = Disk removed.
repair-skipped = Skipped.
repair-complete = Repair complete.

# Solid commands
solid-set = Solid marker set on: {$path}
solid-set-desc = This directory will not be split during storage operations.
solid-unset = Solid marker removed from: {$path}

# Configuration commands
config-current-lang = Current language
config-available-langs = Available languages
config-lang-set = Language set to: {$lang}
config-usage = Usage

# Indexing
index-updated = Run 'disco scan --all' to update the index with new files.