<#
.SYNOPSIS
    Build script for perplexity-web-api-mcp on Windows.

.DESCRIPTION
    Installs required build dependencies (CMake, NASM, Ninja, LLVM/libclang)
    into a local .build-tools directory and builds the project in release mode.

    All paths are dynamic — nothing is hardcoded to a specific user directory.

.PARAMETER Clean
    Remove .build-tools and cargo target directory before building.

.PARAMETER SkipDeps
    Skip installing build dependencies (use if already installed system-wide).
#>

param(
    [switch]$Clean,
    [switch]$SkipDeps
)

$ErrorActionPreference = "Stop"

$ProjectRoot = $PSScriptRoot
$BuildToolsDir = Join-Path $ProjectRoot ".build-tools"
$CmakeDir = Join-Path $BuildToolsDir "cmake"
$NasmDir = Join-Path $BuildToolsDir "nasm"
$NinjaDir = Join-Path $BuildToolsDir "ninja"
$LlvmDir = Join-Path $BuildToolsDir "llvm"

if ($Clean) {
    Write-Host "Cleaning build artifacts..." -ForegroundColor Yellow
    if (Test-Path $BuildToolsDir) { Remove-Item -Recurse -Force $BuildToolsDir }
    if (Test-Path (Join-Path $ProjectRoot "target")) { Remove-Item -Recurse -Force (Join-Path $ProjectRoot "target") }
}

# --- Check prerequisites ---

# Rust/Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "ERROR: Rust is not installed. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Visual Studio Build Tools (MSVC)
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vsWhere) {
    $vsPath = & $vsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    if (-not $vsPath) {
        Write-Host "ERROR: Visual Studio Build Tools with C++ workload not found." -ForegroundColor Red
        Write-Host "Install from: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor Yellow
        exit 1
    }
    Write-Host "Found Visual Studio at: $vsPath" -ForegroundColor Green
} else {
    Write-Host "WARNING: vswhere not found. MSVC may not be detected correctly." -ForegroundColor Yellow
}

if (-not $SkipDeps) {
    New-Item -ItemType Directory -Force -Path $BuildToolsDir | Out-Null

    # --- CMake ---
    if (-not (Test-Path (Join-Path $CmakeDir "bin\cmake.exe"))) {
        Write-Host "Downloading CMake..." -ForegroundColor Cyan
        $cmakeVersion = "3.31.5"
        $cmakeZip = Join-Path $BuildToolsDir "cmake.zip"
        $cmakeUrl = "https://github.com/Kitware/CMake/releases/download/v${cmakeVersion}/cmake-${cmakeVersion}-windows-x86_64.zip"
        Invoke-WebRequest -Uri $cmakeUrl -OutFile $cmakeZip
        Expand-Archive -Path $cmakeZip -DestinationPath $BuildToolsDir -Force
        $cmakeExtracted = Join-Path $BuildToolsDir "cmake-${cmakeVersion}-windows-x86_64"
        if (Test-Path $CmakeDir) { Remove-Item -Recurse -Force $CmakeDir }
        Rename-Item $cmakeExtracted $CmakeDir
        Remove-Item $cmakeZip
        Write-Host "CMake installed." -ForegroundColor Green
    } else {
        Write-Host "CMake already present." -ForegroundColor Green
    }

    # --- NASM ---
    if (-not (Test-Path (Join-Path $NasmDir "nasm.exe"))) {
        Write-Host "Downloading NASM..." -ForegroundColor Cyan
        $nasmVersion = "2.16.03"
        $nasmZip = Join-Path $BuildToolsDir "nasm.zip"
        $nasmUrl = "https://www.nasm.us/pub/nasm/releasebuilds/${nasmVersion}/win64/nasm-${nasmVersion}-win64.zip"
        Invoke-WebRequest -Uri $nasmUrl -OutFile $nasmZip
        Expand-Archive -Path $nasmZip -DestinationPath $BuildToolsDir -Force
        $nasmExtracted = Join-Path $BuildToolsDir "nasm-${nasmVersion}"
        if (Test-Path $NasmDir) { Remove-Item -Recurse -Force $NasmDir }
        Rename-Item $nasmExtracted $NasmDir
        Remove-Item $nasmZip
        Write-Host "NASM installed." -ForegroundColor Green
    } else {
        Write-Host "NASM already present." -ForegroundColor Green
    }

    # --- Ninja ---
    if (-not (Test-Path (Join-Path $NinjaDir "ninja.exe"))) {
        Write-Host "Downloading Ninja..." -ForegroundColor Cyan
        $ninjaVersion = "1.12.1"
        $ninjaZip = Join-Path $BuildToolsDir "ninja.zip"
        $ninjaUrl = "https://github.com/ninja-build/ninja/releases/download/v${ninjaVersion}/ninja-win.zip"
        Invoke-WebRequest -Uri $ninjaUrl -OutFile $ninjaZip
        New-Item -ItemType Directory -Force -Path $NinjaDir | Out-Null
        Expand-Archive -Path $ninjaZip -DestinationPath $NinjaDir -Force
        Remove-Item $ninjaZip
        Write-Host "Ninja installed." -ForegroundColor Green
    } else {
        Write-Host "Ninja already present." -ForegroundColor Green
    }

    # --- LLVM/libclang ---
    if (-not (Test-Path (Join-Path $LlvmDir "bin\libclang.dll"))) {
        Write-Host "Downloading LLVM (for libclang)..." -ForegroundColor Cyan
        $llvmVersion = "18.1.8"
        $llvmExe = Join-Path $BuildToolsDir "llvm-installer.exe"
        $llvmUrl = "https://github.com/llvm/llvm-project/releases/download/llvmorg-${llvmVersion}/LLVM-${llvmVersion}-win64.exe"
        Invoke-WebRequest -Uri $llvmUrl -OutFile $llvmExe

        # Extract using 7-Zip (required for non-admin extraction)
        $sevenZip = "${env:ProgramFiles}\7-Zip\7z.exe"
        if (Test-Path $sevenZip) {
            Write-Host "Extracting LLVM with 7-Zip..." -ForegroundColor Cyan
            New-Item -ItemType Directory -Force -Path $LlvmDir | Out-Null
            # Use a variable for the -o flag to handle paths with spaces correctly
            $llvmOutputArg = "-o$LlvmDir"
            & "$sevenZip" x "$llvmExe" $llvmOutputArg -y | Out-Null
        } else {
            Remove-Item $llvmExe -ErrorAction SilentlyContinue
            Write-Host "ERROR: 7-Zip is required to extract LLVM without admin privileges." -ForegroundColor Red
            Write-Host "Install 7-Zip from: https://7-zip.org" -ForegroundColor Yellow
            Write-Host "Expected path: $sevenZip" -ForegroundColor Yellow
            exit 1
        }
        Remove-Item $llvmExe -ErrorAction SilentlyContinue
        Write-Host "LLVM installed." -ForegroundColor Green
    } else {
        Write-Host "LLVM/libclang already present." -ForegroundColor Green
    }
}

# --- Set environment for the build ---
$env:CMAKE_GENERATOR = "Ninja"

if (-not $SkipDeps) {
    $env:PATH = "$(Join-Path $CmakeDir 'bin');$(Join-Path $NasmDir '');$(Join-Path $NinjaDir '');$env:PATH"
    $env:LIBCLANG_PATH = Join-Path $LlvmDir "bin"

    # Warn if any paths contain spaces, which can cause issues with some build tools
    if ($env:LIBCLANG_PATH -match ' ') {
        Write-Host "WARNING: LIBCLANG_PATH contains spaces: $($env:LIBCLANG_PATH)" -ForegroundColor Yellow
        Write-Host "         If the build fails, set CARGO_HOME to a path without spaces," -ForegroundColor Yellow
        Write-Host "         e.g. set CARGO_HOME=C:\Cargo in your environment variables." -ForegroundColor Yellow
        Write-Host ""
    }

    Write-Host ""
    Write-Host "Build environment:" -ForegroundColor Cyan
    Write-Host "  CMake:    $(Join-Path $CmakeDir 'bin\cmake.exe')"
    Write-Host "  NASM:     $(Join-Path $NasmDir 'nasm.exe')"
    Write-Host "  Ninja:    $(Join-Path $NinjaDir 'ninja.exe')"
    Write-Host "  LLVM:     $($env:LIBCLANG_PATH)"
    Write-Host "  Generator: $($env:CMAKE_GENERATOR)"
    Write-Host ""
}

# --- Build in release mode ---
# Release mode is required to avoid CRT mismatch between BoringSSL (debug /MDd)
# and Rust (release /MD) which causes __imp__CrtDbgReport linker errors.
Write-Host "Building in release mode..." -ForegroundColor Cyan
Push-Location $ProjectRoot
try {
    cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed with exit code $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
    Write-Host ""
    Write-Host "Build successful!" -ForegroundColor Green
    $binaryPath = Join-Path $ProjectRoot "target\release\perplexity-web-api-mcp.exe"
    if (Test-Path $binaryPath) {
        $size = [math]::Round((Get-Item $binaryPath).Length / 1MB, 1)
        Write-Host "Binary: $binaryPath ($size MB)" -ForegroundColor Green
    }
} finally {
    Pop-Location
}
