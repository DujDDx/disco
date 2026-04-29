#
# Disco - One-line Install Script for Windows
# Usage: irm https://raw.githubusercontent.com/Dujddx/disco/main/install.ps1 | iex
#

[CmdletBinding()]
param(
    [switch]$Help,
    [switch]$Uninstall,
    [string]$Prefix,
    [switch]$Verbose
)

# ============================================================================
# Configuration
# ============================================================================

$RepoUrl = "https://github.com/Dujddx/disco"
$BinaryName = "disco.exe"
$DefaultInstallDir = "$env:LOCALAPPDATA\Disco"
$SystemBinDir = "$env:ProgramFiles\Disco"

# ============================================================================
# Colors
# ============================================================================

function Write-Info($message) {
    Write-Host "➜ " -ForegroundColor Blue -NoNewline
    Write-Host $message
}

function Write-Success($message) {
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $message
}

function Write-Warning($message) {
    Write-Host "⚠ " -ForegroundColor Yellow -NoNewline
    Write-Host $message
}

function Write-Error($message) {
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host $message
}

# ============================================================================
# Help
# ============================================================================

function Show-Help {
    Write-Host ""
    Write-Host "Disco Installer for Windows" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  irm https://raw.githubusercontent.com/Dujddx/disco/main/install.ps1 | iex"
    Write-Host "  .\install.ps1 [-Help] [-Uninstall] [-Prefix <path>] [-Verbose]"
    Write-Host ""
    Write-Host "Options:" -ForegroundColor Yellow
    Write-Host "  -Help              Show this help message"
    Write-Host "  -Uninstall         Uninstall Disco"
    Write-Host "  -Prefix <path>     Custom install path (default: $env:LOCALAPPDATA\Disco)"
    Write-Host "  -Verbose           Enable verbose output"
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  # One-line install"
    Write-Host "  irm https://raw.githubusercontent.com/Dujddx/disco/main/install.ps1 | iex"
    Write-Host ""
    Write-Host "  # Custom install path"
    Write-Host "  .\install.ps1 -Prefix 'C:\Tools\Disco'"
    Write-Host ""
    Write-Host "  # Uninstall"
    Write-Host "  .\install.ps1 -Uninstall"
    Write-Host ""
    Write-Host "More info: $RepoUrl"
    exit 0
}

# ============================================================================
# System Check
# ============================================================================

function Check-System {
    Write-Info "Checking system environment..."
    
    $OS = [System.Environment]::OSVersion.Platform
    if ($OS -ne "Win32NT") {
        Write-Error "This script only supports Windows"
        exit 1
    }
    
    # Check architecture
    $Arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    if ($Arch -ne "AMD64") {
        Write-Warning "This script is optimized for x64 architecture"
        Write-Warning "Current architecture: $Arch"
    }
    
    Write-Success "Windows $Arch - System check passed"
}

# ============================================================================
# Rust Check
# ============================================================================

function Check-Rust {
    Write-Info "Checking Rust installation..."
    
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    
    if ($rustc -and $cargo) {
        $version = & rustc --version
        Write-Success "Rust is installed: $version"
        return
    }
    
    Write-Warning "Rust is not installed, installing..."
    
    # Check if rustup-init.exe exists
    $rustupPath = "$env:TEMP\rustup-init.exe"
    
    Write-Info "Downloading rustup-init.exe..."
    try {
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupPath -UseBasicParsing
    } catch {
        Write-Error "Failed to download rustup: $_"
        exit 1
    }
    
    Write-Info "Installing Rust (this may take a few minutes)..."
    try {
        & $rustupPath -y --default-toolchain stable | Out-Null
    } catch {
        Write-Error "Failed to install Rust: $_"
        exit 1
    }
    
    # Refresh environment variables
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    
    # Verify installation
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Error "Rust installation failed"
        exit 1
    }
    
    $version = & rustc --version
    Write-Success "Rust installed: $version"
}

# ============================================================================
# VS Build Tools Check
# ============================================================================

function Check-VSBuildTools {
    Write-Info "Checking Visual Studio Build Tools..."

    # Method 1: Check using vswhere.exe
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vsWhere) {
        $vsPath = & $vsWhere -latest -property installationPath 2>$null
        if ($vsPath) {
            Write-Success "Visual Studio found: $vsPath"
            return $true
        }
    }

    # Method 2: Check for MSVC link.exe in common paths
    $programFilesX86 = ${env:ProgramFiles(x86)}
    $vcvarsallPaths = @(
        "$programFilesX86\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2019\Community\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2019\Professional\VC\Auxiliary\Build\vcvarsall.bat"
        "$programFilesX86\Microsoft Visual Studio\2019\Enterprise\VC\Auxiliary\Build\vcvarsall.bat"
    )

    foreach ($path in $vcvarsallPaths) {
        if (Test-Path $path) {
            Write-Success "Visual Studio Build Tools found: $path"
            return $true
        }
    }

    # Method 3: Check registry for VS Build Tools
    $regPaths = @(
        "HKLM:\SOFTWARE\Microsoft\VisualStudio\17.0\VC",
        "HKLM:\SOFTWARE\Microsoft\VisualStudio\16.0\VC",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\VisualStudio\17.0\VC",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\VisualStudio\16.0\VC"
    )

    foreach ($regPath in $regPaths) {
        if (Test-Path $regPath) {
            Write-Success "Visual Studio found in registry"
            return $true
        }
    }

    # Not found - auto install (no interactive prompt for remote execution)
    Write-Warning "Visual Studio Build Tools not found"
    Write-Warning "Rust MSVC toolchain requires Visual Studio Build Tools with C++ workload"
    Write-Host ""
    Write-Host "Without VS Build Tools, you may encounter errors like:" -ForegroundColor Yellow
    Write-Host "  'link.exe failed with exit code: 1'" -ForegroundColor Gray
    Write-Host "  'link --help' for more information (wrong link command)" -ForegroundColor Gray
    Write-Host ""

    Write-Info "Auto-installing Visual Studio Build Tools..."
    Install-VSBuildTools
    return $true
}

function Install-VSBuildTools {
    Write-Info "Installing Visual Studio Build Tools..."

    # Check if winget is available
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if (-not $winget) {
        Write-Error "winget is not available"
        Write-Info "Please install Visual Studio Build Tools manually from:"
        Write-Info "https://visualstudio.microsoft.com/visual-cpp-build-tools/"
        return $false
    }

    Write-Info "This will install Microsoft Visual Studio Build Tools 2022 with C++ workload"
    Write-Info "This may take 10-30 minutes depending on your system..."
    Write-Host ""

    try {
        $installArgs = @(
            "install",
            "Microsoft.VisualStudio.2022.BuildTools",
            "--override",
            "'--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'"
        )

        Write-Info "Running: winget $($installArgs -join ' ')"
        & winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

        if ($LASTEXITCODE -eq 0) {
            Write-Success "Visual Studio Build Tools installed successfully"
            Write-Warning "You may need to restart your terminal for changes to take effect"
            return $true
        } else {
            Write-Warning "Installation returned exit code: $LASTEXITCODE"
            Write-Info "Please check if VS Build Tools was installed correctly"
            return $false
        }
    } catch {
        Write-Error "Failed to install VS Build Tools: $_"
        Write-Info "Please install manually from:"
        Write-Info "https://visualstudio.microsoft.com/visual-cpp-build-tools/"
        return $false
    }
}

# ============================================================================
# Git Check
# ============================================================================

function Check-Git {
    Write-Info "Checking Git installation..."
    
    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git) {
        Write-Error "Git is not installed"
        Write-Info "Please install Git from: https://git-scm.com/download/win"
        Write-Info "Or use: winget install Git.Git"
        exit 1
    }
    
    $version = & git --version
    Write-Success "Git is installed: $version"
}

# ============================================================================
# Install Path
# ============================================================================

function Get-InstallPath {
    if ($Prefix) {
        $script:InstallDir = $Prefix
    } elseif (Test-Path "$env:ProgramFiles\Disco" -PathType Container) {
        $script:InstallDir = "$env:ProgramFiles\Disco"
    } else {
        $script:InstallDir = $DefaultInstallDir
    }
    
    Write-Info "Install path: $script:InstallDir"
    
    if (-not (Test-Path $script:InstallDir)) {
        New-Item -ItemType Directory -Path $script:InstallDir -Force | Out-Null
        Write-Success "Created directory: $script:InstallDir"
    }
}

function Add-ToPath {
    $currentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    
    if ($currentPath -notlike "*$script:InstallDir*") {
        [System.Environment]::SetEnvironmentVariable("Path", "$currentPath;$script:InstallDir", "User")
        Write-Success "Added to PATH: $script:InstallDir"
        Write-Warning "Please restart your terminal or run: `$env:Path = [System.Environment]::GetEnvironmentVariable('Path', 'User')"
    }
}

# ============================================================================
# Install
# ============================================================================

function Install-Disco {
    Write-Info "Starting Disco installation..."
    
    # Create temp directory
    $tempDir = New-TemporaryDirectory
    Write-Info "Using temp directory: $tempDir"
    
    try {
        Write-Info "Cloning repository..."
        & git clone --depth 1 $RepoUrl $tempDir
        
        Push-Location $tempDir
        
        Write-Info "Building release version (this may take a few minutes)..."

        if ($Verbose) {
            & cargo build --release
            $buildExitCode = $LASTEXITCODE
        } else {
            $buildOutput = & cargo build --release 2>&1
            $buildExitCode = $LASTEXITCODE
        }

        if ($buildExitCode -ne 0) {
            Write-Error "Build failed with exit code: $buildExitCode"
            if (-not $Verbose) {
                Write-Info "Build output:"
                Write-Host $buildOutput
            }
            exit 1
        }

        $binaryPath = Join-Path $tempDir "target\release\$BinaryName"

        if (-not (Test-Path $binaryPath)) {
            Write-Error "Build failed: binary not found at $binaryPath"
            Write-Warning "This may indicate a build configuration issue"
            exit 1
        }

        Write-Success "Build completed"
        
        # Install
        Write-Info "Installing to $script:InstallDir..."
        Copy-Item $binaryPath $script:InstallDir -Force
        
        Write-Success "Installation complete: $(Join-Path $script:InstallDir $BinaryName)"
        
        # Add to PATH
        Add-ToPath
        
        # Show success
        Show-SuccessInfo
        
    } finally {
        Pop-Location
        Write-Info "Cleaning up temp files..."
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ============================================================================
# Uninstall
# ============================================================================

function Uninstall-Disco {
    Write-Info "Uninstalling Disco..."
    
    $found = $false
    
    $dirs = @($DefaultInstallDir, $SystemBinDir, "$env:USERPROFILE\.cargo\bin")
    
    foreach ($dir in $dirs) {
        $binary = Join-Path $dir $BinaryName
        if (Test-Path $binary) {
            Remove-Item $binary -Force
            Write-Success "Deleted: $binary"
            $found = $true
        }
    }
    
    # Also check for disco without .exe extension
    foreach ($dir in $dirs) {
        $binary = Join-Path $dir "disco"
        if (Test-Path $binary) {
            Remove-Item $binary -Force
            Write-Success "Deleted: $binary"
            $found = $true
        }
    }
    
    # Remove from PATH
    $currentPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -like "*$DefaultInstallDir*") {
        $newPath = ($currentPath -split ';' | Where-Object { $_ -ne $DefaultInstallDir }) -join ';'
        [System.Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Success "Removed from PATH"
    }
    
    if (-not $found) {
        Write-Warning "Disco not found"
    } else {
        Write-Success "Uninstall complete"
    }
    
    exit 0
}

# ============================================================================
# Helper Functions
# ============================================================================

function New-TemporaryDirectory {
    $tempPath = [System.IO.Path]::GetTempPath()
    $tempDir = [System.IO.Path]::Combine($tempPath, [System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    return $tempDir
}

function Show-SuccessInfo {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║           Disco Installed Successfully!              ║" -ForegroundColor Green
    Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "Install location: $(Join-Path $script:InstallDir $BinaryName)"
    Write-Host ""
    
    $disco = Get-Command disco -ErrorAction SilentlyContinue
    if ($disco) {
        Write-Host "Version:" -ForegroundColor Yellow
        & disco --version 2>$null
        Write-Host ""
        Write-Host "Usage:" -ForegroundColor Yellow
        Write-Host "  disco --help     Show help message"
        Write-Host "  disco search     Search files"
        Write-Host "  disco store      Storage management"
        Write-Host ""
        Write-Host "You can now run 'disco' command!" -ForegroundColor Green
    } else {
        Write-Host "Note: $script:InstallDir is not in PATH yet" -ForegroundColor Yellow
        Write-Host "Please restart your terminal or run:"
        Write-Host ""
        Write-Host "  `$env:Path += ';$script:InstallDir'" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Then run:"
        Write-Host "  disco --help" -ForegroundColor Cyan
    }
    Write-Host ""
    Write-Host "More info: $RepoUrl"
    Write-Host ""
}

# ============================================================================
# Main
# ============================================================================

if ($Help) {
    Show-Help
}

if ($Uninstall) {
    Uninstall-Disco
}

function Main {
    Write-Host ""
    Write-Host "╔══════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║            Disco Installer for Windows               ║" -ForegroundColor Cyan
    Write-Host "╚══════════════════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""

    Check-System
    Check-Git
    Check-VSBuildTools
    Check-Rust
    Get-InstallPath
    Install-Disco
}

Main
