# Menu strings for Disco CLI

# Main menu items
menu-disk-management = Disk Management
menu-scan-files = Scan Files
menu-search-files = Search Files
menu-store-files = Store Files
menu-retrieve-files = Retrieve Files
menu-view-status = View Status
menu-refresh-status = Refresh Status
menu-repair-offline = Repair Offline
menu-visualize = Visualize
menu-settings = Settings
menu-exit = Exit Menu

# Submenu - Disk Management
submenu-disk-title = Disk Management
submenu-disk-add = Add Disk
submenu-disk-list = List Disks
submenu-disk-add-prompt = Enter mount point:

# Submenu - Scan
submenu-scan-title = Scan Files
submenu-scan-all = Scan all disks
submenu-scan-specific = Scan specific disk
submenu-scan-select-disk = Select Disk

# Submenu - Settings
submenu-settings-title = Settings
submenu-settings-lang = Language
submenu-settings-lang-desc = Press Enter or Left/Right to switch language
submenu-settings-hash = Hash Verification
submenu-settings-hash-on = ON
submenu-settings-hash-off = OFF
submenu-settings-hash-desc = When enabled, file hashes are calculated during scan and store

# Menu help text
menu-help-title = DISCO
menu-help-navigate = ↑/↓: Navigate
menu-help-select = Enter: Select
menu-help-quick = 1-9,0,q: Quick
menu-help-exit = Esc: Exit
menu-help-back = Backspace: Back

# Menu footer
menu-returned = Returned to command mode.
menu-back = Back
menu-no-disks = No disks registered.
menu-error = Error:
menu-press-enter = Press Enter to continue...

# Menu descriptions (second column)
menu-desc-disk = Add, list, rename, remove disks
menu-desc-scan = Scan disks for files
menu-desc-search = Search indexed files
menu-desc-store = Store files to disks
menu-desc-retrieve = Retrieve files from disks
menu-desc-status = Show disk status overview
menu-desc-refresh = Force refresh mount detection
menu-desc-repair = Fix offline disk identities
menu-desc-visualize = Open TUI visualization
menu-desc-settings = Configure hash verification
menu-desc-exit = Return to command mode

# Usage warnings
usage-disk = Usage: disk <add|list|rename|remove>
usage-disk-add = Usage: disk add <mount-point> [--name N]
usage-disk-rename = Usage: disk rename <disk-id> <new-name>
usage-disk-remove = Usage: disk remove <disk-id>
usage-scan = Usage: scan [--all] [--disk D] [--hash] [--full]
usage-search = Usage: search <keyword> [--ext E] [--limit N]
usage-get = Usage: get <entry-id> [--locate]
usage-store = Usage: store <paths...> [--solid-layer S]
usage-retrieve = Usage: retrieve <keyword>
usage-solid = Usage: solid <set|unset> <path> [--disk D]

# Unknown command
unknown-command = Unknown command: {$command}
unknown-disk-subcommand = Unknown disk subcommand: {$command}
unknown-solid-subcommand = Unknown solid subcommand: {$command}
available-disk-commands = Available: add, list, rename, remove

# Prompts
prompt-enter-mount-point = Enter mount point:
prompt-enter-keyword = Enter keyword:
prompt-enter-paths = Enter paths (comma separated):
prompt-enter-solid-layer = Enter solid layer depth [0]:
prompt-enter-destination = Enter destination directory [default: ./]:
prompt-enter-disk-id = Enter disk ID:
prompt-select-files = Enter file numbers to retrieve (e.g., 1,3,5) or 'all':