#!/bin/bash
#
# Disco - One-line Install Script for Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh
#

set -e

# ============================================================================
# Configuration
# ============================================================================

REPO_URL="https://github.com/Dujddx/disco"
BINARY_NAME="disco"
INSTALL_VERSION="release"

# Default install paths
DEFAULT_LOCAL_BIN="$HOME/.local/bin"
SYSTEM_BIN="/usr/local/bin"

# ============================================================================
# Colors
# ============================================================================

if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    BOLD=''
    RESET=''
fi

info() {
    echo -e "${BLUE}➜${RESET} $1"
}

success() {
    echo -e "${GREEN}✓${RESET} $1"
}

warn() {
    echo -e "${YELLOW}⚠${RESET} $1"
}

error() {
    echo -e "${RED}✗${RESET} $1" >&2
}

bold() {
    echo -e "${BOLD}$1${RESET}"
}

# ============================================================================
# Help
# ============================================================================

show_help() {
    cat << EOF
${BOLD}Disco Installer for Linux${RESET} - One-line installation tool

${BOLD}Usage:${RESET}
    curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh
    ./install-linux.sh [options]

${BOLD}Options:${RESET}
    -h, --help          Show this help message
    --uninstall         Uninstall Disco
    --prefix <path>     Custom install path (default: ~/.local/bin or /usr/local/bin)
    --verbose           Enable verbose output

${BOLD}Examples:${RESET}
    # One-line install
    curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install-linux.sh | sh

    # Custom install path
    ./install-linux.sh --prefix /usr/local/bin

    # Uninstall
    ./install-linux.sh --uninstall

${BOLD}Supported distributions:${RESET}
    - Debian/Ubuntu (apt)
    - Fedora/RHEL/CentOS (dnf/yum)
    - Arch Linux (pacman)
    - openSUSE (zypper)
    - Alpine (apk)

${BOLD}More info:${RESET} ${REPO_URL}
EOF
    exit 0
}

# ============================================================================
# System Detection
# ============================================================================

detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO="$ID"
        DISTRO_VERSION="$VERSION_ID"
    elif [ -f /etc/redhat-release ]; then
        DISTRO="rhel"
    elif [ -f /etc/arch-release ]; then
        DISTRO="arch"
    elif [ -f /etc/debian_version ]; then
        DISTRO="debian"
    else
        DISTRO="unknown"
    fi
}

check_system() {
    info "Detecting system environment..."

    # Detect OS
    OS="$(uname -s)"
    if [ "$OS" != "Linux" ]; then
        error "This script only supports Linux"
        exit 1
    fi

    # Detect architecture
    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64)
            ARCH_NAME="x86_64"
            ;;
        aarch64|arm64)
            ARCH_NAME="aarch64"
            ;;
        armv7l|armhf)
            ARCH_NAME="armv7"
            warn "ARMv7 architecture may have limited performance"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    # Detect distribution
    detect_distro

    success "Linux $ARCH_NAME ($DISTRO) - System check passed"
}

# ============================================================================
# Dependency Installation
# ============================================================================

install_dependencies() {
    info "Checking dependencies..."

    # Check for build-essential or equivalent
    local need_build_tools=false
    local need_git=false
    local need_curl=false

    if ! command -v make &> /dev/null || ! command -v gcc &> /dev/null; then
        need_build_tools=true
    fi

    if ! command -v git &> /dev/null; then
        need_git=true
    fi

    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        need_curl=true
    fi

    if [ "$need_build_tools" = false ] && [ "$need_git" = false ] && [ "$need_curl" = false ]; then
        success "All dependencies are installed"
        return 0
    fi

    warn "Missing dependencies, installing..."

    case "$DISTRO" in
        ubuntu|debian|linuxmint|pop)
            sudo apt-get update -qq
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs build-essential"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            sudo apt-get install -y $pkgs
            ;;
        fedora)
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs gcc make"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            sudo dnf install -y $pkgs
            ;;
        rhel|centos|rocky|almalinux)
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs gcc make"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            if command -v dnf &> /dev/null; then
                sudo dnf install -y $pkgs
            else
                sudo yum install -y $pkgs
            fi
            ;;
        arch|manjaro|endeavouros)
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs base-devel"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            sudo pacman -S --noconfirm $pkgs
            ;;
        opensuse-leap|opensuse-tumbleweed|opensuse)
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs gcc make"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            sudo zypper install -y $pkgs
            ;;
        alpine)
            local pkgs=""
            [ "$need_build_tools" = true ] && pkgs="$pkgs build-base"
            [ "$need_git" = true ] && pkgs="$pkgs git"
            [ "$need_curl" = true ] && pkgs="$pkgs curl"
            sudo apk add $pkgs
            ;;
        *)
            warn "Unknown distribution: $DISTRO"
            warn "Please install the following manually:"
            [ "$need_build_tools" = true ] && echo "  - build-essential (gcc, make)"
            [ "$need_git" = true ] && echo "  - git"
            [ "$need_curl" = true ] && echo "  - curl or wget"
            return 1
            ;;
    esac

    success "Dependencies installed"
}

# ============================================================================
# Rust Installation
# ============================================================================

check_rust() {
    info "Checking Rust installation..."

    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        success "Rust is installed: $RUST_VERSION"
        return 0
    fi

    warn "Rust is not installed, installing..."

    # Install rustup
    if command -v rustup &> /dev/null; then
        info "rustup exists, updating..."
        rustup update stable
    else
        info "Installing rustup..."

        # Use curl or wget
        if command -v curl &> /dev/null; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        elif command -v wget &> /dev/null; then
            wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        else
            error "Neither curl nor wget is available"
            exit 1
        fi

        # Load cargo environment
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        elif [ -f "$HOME/.bashrc" ]; then
            source "$HOME/.bashrc"
        elif [ -f "$HOME/.zshrc" ]; then
            source "$HOME/.zshrc"
        fi
    fi

    # Verify installation
    if ! command -v cargo &> /dev/null; then
        error "Rust installation failed"
        exit 1
    fi

    success "Rust installed: $(rustc --version)"
}

# ============================================================================
# Install Path
# ============================================================================

determine_install_path() {
    if [ -n "$INSTALL_PREFIX" ]; then
        INSTALL_DIR="$INSTALL_PREFIX"
    elif [ -w "$SYSTEM_BIN" ] || [ "$(id -u)" = "0" ]; then
        INSTALL_DIR="$SYSTEM_BIN"
    else
        INSTALL_DIR="$DEFAULT_LOCAL_BIN"
    fi

    info "Install path: $INSTALL_DIR"

    # Create directory
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
        success "Created directory: $INSTALL_DIR"
    fi
}

add_to_path() {
    local shell_config=""
    local path_line="export PATH=\"\$PATH:$INSTALL_DIR\""

    # Detect current shell
    case "$SHELL" in
        */zsh)
            shell_config="$HOME/.zshrc"
            ;;
        */bash)
            if [ -f "$HOME/.bash_profile" ]; then
                shell_config="$HOME/.bash_profile"
            else
                shell_config="$HOME/.bashrc"
            fi
            ;;
        */fish)
            # Fish shell uses different config format
            local fish_config="$HOME/.config/fish/config.fish"
            if [ -d "$HOME/.config/fish" ] && [ ! -f "$fish_config" ]; then
                mkdir -p "$HOME/.config/fish"
                touch "$fish_config"
            fi
            if [ -f "$fish_config" ] && ! grep -q "$INSTALL_DIR" "$fish_config" 2>/dev/null; then
                echo "set -gx PATH \$PATH $INSTALL_DIR" >> "$fish_config"
                success "Added to Fish PATH: $fish_config"
            fi
            return
            ;;
    esac

    if [ -n "$shell_config" ] && [ -f "$shell_config" ]; then
        if ! grep -q "$INSTALL_DIR" "$shell_config" 2>/dev/null; then
            echo "" >> "$shell_config"
            echo "# Added by Disco installer" >> "$shell_config"
            echo "$path_line" >> "$shell_config"
            success "Added to PATH: $shell_config"
            warn "Please run 'source $shell_config' or restart terminal"
        fi
    fi
}

# ============================================================================
# Install
# ============================================================================

install() {
    info "Starting Disco installation..."

    # Create temp directory
    TEMP_DIR=$(mktemp -d)
    trap "cleanup '$TEMP_DIR'" EXIT

    info "Cloning repository..."
    git clone --depth 1 "$REPO_URL" "$TEMP_DIR"

    cd "$TEMP_DIR"

    info "Building release version (this may take a few minutes)..."
    cargo build --release

    # Check build result
    BINARY_PATH="$TEMP_DIR/target/release/$BINARY_NAME"
    if [ ! -f "$BINARY_PATH" ]; then
        error "Build failed: binary not found"
        exit 1
    fi

    success "Build completed"

    # Install
    info "Installing to $INSTALL_DIR..."
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "Installed: $INSTALL_DIR/$BINARY_NAME"

    # Add to PATH
    add_to_path

    # Show success
    show_success_info
}

# ============================================================================
# Uninstall
# ============================================================================

uninstall() {
    info "Uninstalling Disco..."

    local found=false

    for dir in "$DEFAULT_LOCAL_BIN" "$SYSTEM_BIN" "$HOME/.cargo/bin"; do
        local binary="$dir/$BINARY_NAME"
        if [ -f "$binary" ]; then
            rm -f "$binary"
            success "Deleted: $binary"
            found=true
        fi
    done

    if [ "$found" = false ]; then
        warn "Disco not found"
    else
        success "Uninstall complete"
    fi

    exit 0
}

# ============================================================================
# Cleanup
# ============================================================================

cleanup() {
    local temp_dir="$1"
    if [ -d "$temp_dir" ]; then
        info "Cleaning up temp files..."
        rm -rf "$temp_dir"
    fi
}

# ============================================================================
# Success Info
# ============================================================================

show_success_info() {
    echo ""
    echo -e "${GREEN}${BOLD}╔══════════════════════════════════════════════════════╗${RESET}"
    echo -e "${GREEN}${BOLD}║           Disco Installed Successfully!              ║${RESET}"
    echo -e "${GREEN}${BOLD}╚══════════════════════════════════════════════════════╝${RESET}"
    echo ""
    echo -e "${BOLD}Install location:${RESET} $INSTALL_DIR/$BINARY_NAME"
    echo ""

    # Check if in PATH
    if command -v $BINARY_NAME &> /dev/null; then
        echo -e "${BOLD}Version:${RESET}"
        $BINARY_NAME --version 2>/dev/null || true
        echo ""
        echo -e "${BOLD}Usage:${RESET}"
        echo "  $BINARY_NAME --help     Show help message"
        echo "  $BINARY_NAME search     Search files"
        echo "  $BINARY_NAME store      Storage management"
        echo ""
        echo -e "${GREEN}You can now run 'disco' command!${RESET}"
    else
        echo -e "${YELLOW}Note: $INSTALL_DIR is not in PATH${RESET}"
        echo "Please run the following or restart terminal:"
        echo ""
        echo -e "  ${BOLD}export PATH=\"\$PATH:$INSTALL_DIR\"${RESET}"
        echo ""
        echo "Then run:"
        echo -e "  ${BOLD}disco --help${RESET}"
    fi
    echo ""
    echo -e "${BOLD}More info:${RESET} $REPO_URL"
    echo ""
}

# ============================================================================
# Argument Parsing
# ============================================================================

INSTALL_PREFIX=""
VERBOSE=false

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            show_help
            ;;
        --uninstall)
            uninstall
            ;;
        --prefix)
            shift
            if [ -z "$1" ]; then
                error "--prefix requires a path"
                exit 1
            fi
            INSTALL_PREFIX="$1"
            ;;
        --verbose)
            VERBOSE=true
            set -x
            ;;
        *)
            error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
    shift
done

# ============================================================================
# Main
# ============================================================================

main() {
    echo ""
    echo -e "${BOLD}╔══════════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}║            Disco Installer for Linux                ║${RESET}"
    echo -e "${BOLD}╚══════════════════════════════════════════════════════╝${RESET}"
    echo ""

    check_system
    install_dependencies
    check_rust
    determine_install_path
    install
}

main
