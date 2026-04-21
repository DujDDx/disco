# Help text and command descriptions for Disco CLI

# Main command descriptions
cmd-disk-desc = Manage disk pool entries
cmd-scan-desc = Scan disks and build/update the file index
cmd-search-desc = Search for files in the index
cmd-get-desc = Get a file location by entry ID
cmd-store-desc = Store files/folders into the disk pool
cmd-retrieve-desc = Retrieve files from the disk pool
cmd-solid-desc = Manage Solid markers on directories
cmd-visualize-desc = Open the terminal visualization interface
cmd-menu-desc = Open visual menu mode with arrow key navigation

# Disk subcommands
cmd-disk-add-desc = Add a new disk to the pool
cmd-disk-list-desc = List all registered disks and their status

# Solid subcommands
cmd-solid-set-desc = Set Solid marker on a directory
cmd-solid-unset-desc = Remove Solid marker from a directory

# Argument descriptions
arg-verbose = Increase logging verbosity (-v, -vv, -vvv)
arg-interactive = Start interactive shell mode

# Disk command arguments
arg-disk-mount = Mount point of the disk to add (e.g., /Volumes/MyDisk on macOS)
arg-disk-name = Custom name for the disk (will prompt if not provided)
arg-disk-detailed = Show detailed information including identity details

# Scan command arguments
arg-scan-all = Scan all registered disks
arg-scan-disk = Scan a specific disk by ID or name
arg-scan-hash = Enable hash calculation during scan
arg-scan-full = Force full scan instead of incremental

# Search command arguments
arg-search-keyword = Search keyword (matches file name)
arg-search-min-size = Filter by minimum file size (in bytes)
arg-search-max-size = Filter by maximum file size (in bytes)
arg-search-ext = Filter by file extension (e.g., .pdf)
arg-search-limit = Limit number of results

# Get command arguments
arg-get-id = Entry ID from search results
arg-get-locate = Check if disk is mounted and provide access path

# Store command arguments
arg-store-paths = Paths to store (supports drag-and-drop paths)
arg-store-solid-layer = SolidLayer depth (0=no split, 1=split to first level, inf=split to files)
arg-store-dedup = Enable hash-based deduplication
arg-store-preview = Preview the storage plan without executing
arg-store-yes = Skip confirmation prompt

# Retrieve command arguments
arg-retrieve-keyword = Search keyword to find files
arg-retrieve-dest = Destination directory to retrieve files to
arg-retrieve-ext = File extension filter
arg-retrieve-limit = Maximum number of results to show
arg-retrieve-all = Retrieve all matching files without confirmation
arg-retrieve-folder = Search for folders instead of files

# Solid command arguments
arg-solid-path = Indexed path of the directory to mark
arg-solid-disk = Disk ID or name (required for ambiguous paths)

# Visualize command arguments
arg-viz-usage = Start in disk usage view (treemap)
arg-viz-tree = Start in tree view
arg-viz-disk = Filter to specific disk