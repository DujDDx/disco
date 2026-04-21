#!/bin/bash
#
# Disco - 一键安装脚本 for Apple Silicon MacBook
# Usage: curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh
#

set -e

# ============================================================================
# 配置
# ============================================================================

REPO_URL="https://github.com/Dujddx/disco"
BINARY_NAME="disco"
INSTALL_VERSION="release"

# 默认安装路径
DEFAULT_LOCAL_BIN="$HOME/.local/bin"
SYSTEM_BIN="/usr/local/bin"

# ============================================================================
# 颜色输出
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
# 帮助信息
# ============================================================================

show_help() {
    cat << EOF
${BOLD}Disco 安装脚本${RESET} - Apple Silicon MacBook 一键安装工具

${BOLD}用法:${RESET}
    curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh
    ./install.sh [选项]

${BOLD}选项:${RESET}
    -h, --help          显示帮助信息
    --uninstall         卸载 Disco
    --prefix <路径>     指定安装路径 (默认: ~/.local/bin 或 /usr/local/bin)
    --verbose           显示详细输出

${BOLD}示例:${RESET}
    # 一键安装
    curl -fsSL https://raw.githubusercontent.com/Dujddx/disco/main/install.sh | sh

    # 指定安装路径
    ./install.sh --prefix /usr/local/bin

    # 卸载
    ./install.sh --uninstall

${BOLD}更多信息:${RESET}
    ${REPO_URL}
EOF
    exit 0
}

# ============================================================================
# 系统检测
# ============================================================================

check_system() {
    info "检测系统环境..."

    # 检测操作系统
    OS="$(uname -s)"
    if [ "$OS" != "Darwin" ]; then
        error "此脚本仅支持 macOS (Apple Silicon)"
        exit 1
    fi

    # 检测 CPU 架构
    ARCH="$(uname -m)"
    if [ "$ARCH" != "arm64" ]; then
        error "此脚本仅支持 Apple Silicon (arm64) 架构"
        error "当前架构: $ARCH"
        exit 1
    fi

    success "Apple Silicon Mac (arm64) - 系统检测通过"
}

# ============================================================================
# Rust 安装检查
# ============================================================================

check_rust() {
    info "检查 Rust 安装..."

    if command -v rustc &> /dev/null && command -v cargo &> /dev/null; then
        RUST_VERSION=$(rustc --version)
        success "Rust 已安装: $RUST_VERSION"
        return 0
    fi

    warn "Rust 未安装，正在安装..."

    # 安装 rustup
    if command -v rustup &> /dev/null; then
        info "rustup 已存在，更新..."
        rustup update stable
    else
        info "安装 rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

        # 加载 cargo 环境
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        elif [ -f "$HOME/.zshenv" ]; then
            source "$HOME/.zshenv"
        elif [ -f "$HOME/.bashrc" ]; then
            source "$HOME/.bashrc"
        fi
    fi

    # 再次检查
    if ! command -v cargo &> /dev/null; then
        error "Rust 安装失败"
        exit 1
    fi

    success "Rust 安装完成: $(rustc --version)"
}

# ============================================================================
# 安装路径处理
# ============================================================================

determine_install_path() {
    if [ -n "$INSTALL_PREFIX" ]; then
        INSTALL_DIR="$INSTALL_PREFIX"
    elif [ -w "$SYSTEM_BIN" ]; then
        INSTALL_DIR="$SYSTEM_BIN"
    else
        INSTALL_DIR="$DEFAULT_LOCAL_BIN"
    fi

    info "安装路径: $INSTALL_DIR"

    # 创建目录
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
        success "创建目录: $INSTALL_DIR"
    fi
}

add_to_path() {
    local shell_config=""
    local path_line="export PATH=\"\$PATH:$INSTALL_DIR\""

    # 检测当前 shell
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
            # Fish shell 使用不同的配置方式
            local fish_config="$HOME/.config/fish/config.fish"
            if [ -d "$HOME/.config/fish" ] && [ ! -f "$fish_config" ]; then
                mkdir -p "$HOME/.config/fish"
                touch "$fish_config"
            fi
            if [ -f "$fish_config" ] && ! grep -q "$INSTALL_DIR" "$fish_config" 2>/dev/null; then
                echo "set -gx PATH \$PATH $INSTALL_DIR" >> "$fish_config"
                success "已添加到 Fish PATH: $fish_config"
            fi
            return
            ;;
    esac

    if [ -n "$shell_config" ] && [ -f "$shell_config" ]; then
        if ! grep -q "$INSTALL_DIR" "$shell_config" 2>/dev/null; then
            echo "" >> "$shell_config"
            echo "# Added by Disco installer" >> "$shell_config"
            echo "$path_line" >> "$shell_config"
            success "已添加到 PATH: $shell_config"
            warn "请运行 'source $shell_config' 或重新打开终端以生效"
        fi
    fi
}

# ============================================================================
# 安装逻辑
# ============================================================================

install() {
    info "开始安装 Disco..."

    # 创建临时目录
    TEMP_DIR=$(mktemp -d)
    trap "cleanup '$TEMP_DIR'" EXIT

    info "克隆仓库到临时目录..."
    git clone --depth 1 "$REPO_URL" "$TEMP_DIR"

    cd "$TEMP_DIR"

    info "编译 release 版本 (这可能需要几分钟)..."
    cargo build --release

    # 检查编译结果
    BINARY_PATH="$TEMP_DIR/target/release/$BINARY_NAME"
    if [ ! -f "$BINARY_PATH" ]; then
        error "编译失败: 找不到二进制文件"
        exit 1
    fi

    success "编译完成"

    # 安装
    info "安装到 $INSTALL_DIR..."
    cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    success "安装完成: $INSTALL_DIR/$BINARY_NAME"

    # 添加到 PATH
    add_to_path

    # 显示成功信息
    show_success_info
}

# ============================================================================
# 卸载逻辑
# ============================================================================

uninstall() {
    info "卸载 Disco..."

    local found=false

    for dir in "$DEFAULT_LOCAL_BIN" "$SYSTEM_BIN" "$HOME/.cargo/bin"; do
        local binary="$dir/$BINARY_NAME"
        if [ -f "$binary" ]; then
            rm -f "$binary"
            success "已删除: $binary"
            found=true
        fi
    done

    if [ "$found" = false ]; then
        warn "未找到已安装的 Disco"
    else
        success "卸载完成"
    fi

    exit 0
}

# ============================================================================
# 清理
# ============================================================================

cleanup() {
    local temp_dir="$1"
    if [ -d "$temp_dir" ]; then
        info "清理临时文件..."
        rm -rf "$temp_dir"
    fi
}

# ============================================================================
# 成功信息
# ============================================================================

show_success_info() {
    echo ""
    echo -e "${GREEN}${BOLD}╔══════════════════════════════════════════════════════╗${RESET}"
    echo -e "${GREEN}${BOLD}║           Disco 安装成功!                            ║${RESET}"
    echo -e "${GREEN}${BOLD}╚══════════════════════════════════════════════════════╝${RESET}"
    echo ""
    echo -e "${BOLD}安装位置:${RESET} $INSTALL_DIR/$BINARY_NAME"
    echo ""

    # 检查是否在 PATH 中
    if command -v $BINARY_NAME &> /dev/null; then
        echo -e "${BOLD}版本信息:${RESET}"
        $BINARY_NAME --version 2>/dev/null || true
        echo ""
        echo -e "${BOLD}使用方法:${RESET}"
        echo "  $BINARY_NAME --help     显示帮助信息"
        echo "  $BINARY_NAME search     搜索文件"
        echo "  $BINARY_NAME store      存储管理"
        echo ""
        echo -e "${GREEN}现在可以直接运行 'disco' 命令了!${RESET}"
    else
        echo -e "${YELLOW}注意: $INSTALL_DIR 不在 PATH 中${RESET}"
        echo -e "请运行以下命令或重新打开终端:"
        echo ""
        echo -e "  ${BOLD}export PATH=\"\$PATH:$INSTALL_DIR\"${RESET}"
        echo ""
        echo -e "然后运行:"
        echo -e "  ${BOLD}disco --help${RESET}"
    fi
    echo ""
    echo -e "${BOLD}更多信息:${RESET} $REPO_URL"
    echo ""
}

# ============================================================================
# 参数解析
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
                error "--prefix 需要指定路径"
                exit 1
            fi
            INSTALL_PREFIX="$1"
            ;;
        --verbose)
            VERBOSE=true
            set -x
            ;;
        *)
            error "未知选项: $1"
            echo "使用 --help 查看帮助信息"
            exit 1
            ;;
    esac
    shift
done

# ============================================================================
# 主流程
# ============================================================================

main() {
    echo ""
    echo -e "${BOLD}╔══════════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}║         Disco Installer for Apple Silicon           ║${RESET}"
    echo -e "${BOLD}╚══════════════════════════════════════════════════════╝${RESET}"
    echo ""

    check_system
    check_rust
    determine_install_path
    install
}

main
