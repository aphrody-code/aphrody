<#
.SYNOPSIS
    aphrody — Windows 11 Insider Canary setup (priority #2 target)

.DESCRIPTION
    Installs all dependencies needed to build, test, and audit the `aphrody`
    CLI binary natively on Windows 11 Insider Canary Build.

    Mirrors scripts/setup-linux.sh — same toolchain, same versions, idempotent.

.PARAMETER Mode
    install   Full install (default)
    Tools     Cargo tools only (skip winget)
    Check     Verify without installing

.EXAMPLE
    pwsh scripts/setup-win.ps1
    pwsh scripts/setup-win.ps1 -Mode Check
    pwsh scripts/setup-win.ps1 -Mode Tools

.NOTES
    Pre-requires PowerShell 7+ and winget. Reads .config/configuration.winget
    for the curated package catalog (VS 2026 Insiders, SDK 26100, Ninja, NASM…).
#>
[CmdletBinding()]
param(
    [ValidateSet('install', 'Tools', 'Check')]
    [string]$Mode = 'install'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 3.0

$RepoRoot = (Split-Path -Parent $PSScriptRoot)

function Write-Info($m) { Write-Host "▶ $m" -ForegroundColor Cyan }
function Write-Ok($m)   { Write-Host "  ✓ $m" -ForegroundColor Green }
function Write-Fail($m) { Write-Host "  ✗ $m" -ForegroundColor Red }
function Write-Warn($m) { Write-Host "  ⚠ $m" -ForegroundColor Yellow }

# -----------------------------------------------------------------------------
# 1. WinGet configuration (catalog: VS 2026 Insiders, SDK 26100, etc.)
# -----------------------------------------------------------------------------
function Install-WinGetCatalog {
    $configPath = Join-Path $RepoRoot '.config\configuration.winget'
    if (-not (Test-Path -LiteralPath $configPath)) {
        Write-Warn "winget config not found at $configPath"
        return
    }
    Write-Info "Applying winget catalog: $configPath"
    winget configure $configPath --accept-configuration-agreements
    Write-Ok 'winget catalog applied'
}

# -----------------------------------------------------------------------------
# 2. Bun (JS/TS runtime — node forbidden)
# -----------------------------------------------------------------------------
function Install-Bun {
    if (Get-Command bun -ErrorAction SilentlyContinue) {
        Write-Ok "bun already present ($(bun --version))"
        return
    }
    Write-Info 'Installing Bun'
    powershell -ExecutionPolicy Bypass -Command "irm bun.sh/install.ps1 | iex"
    $env:Path = "$env:USERPROFILE\.bun\bin;$env:Path"
    Write-Ok "bun installed ($(bun --version))"
}

# -----------------------------------------------------------------------------
# 3. Rust toolchain (nightly, pinned via rust-toolchain.toml)
# -----------------------------------------------------------------------------
function Install-Rust {
    if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
        Write-Info 'Installing rustup'
        $rustupInit = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest 'https://win.rustup.rs/x86_64' -OutFile $rustupInit
        & $rustupInit -y --no-modify-path --default-toolchain none
        $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    }

    Write-Info 'Installing nightly toolchain + components'
    rustup toolchain install nightly --component clippy --component rustfmt --component rust-src --component miri
    rustup default nightly
    Write-Ok "rust toolchain ready ($(rustc --version))"

    Write-Info 'Adding cross-platform targets'
    rustup target add `
        x86_64-pc-windows-msvc `
        x86_64-unknown-linux-gnu `
        wasm32-wasi `
        wasm32-unknown-unknown
    Write-Ok 'targets installed'
}

# -----------------------------------------------------------------------------
# 4. Cargo tooling
# -----------------------------------------------------------------------------
$CargoTools = @(
    'sccache',
    'cargo-nextest',
    'cargo-deny',
    'cargo-vet',
    'cargo-machete',
    'cargo-auditable',
    'cargo-zigbuild',
    'wasm-bindgen-cli',
    'wasm-pack'
)

function Install-CargoTools {
    Write-Info 'Installing cargo tools'
    $installed = & cargo install --list 2>$null | Select-String -Pattern '^\w'
    foreach ($tool in $CargoTools) {
        if ($installed -match "^$tool ") {
            Write-Ok "$tool already installed"
        }
        else {
            try {
                cargo install --locked $tool
                Write-Ok "installed $tool"
            }
            catch {
                Write-Warn "failed: $tool (continuing)"
            }
        }
    }
}

# -----------------------------------------------------------------------------
# 5. Verification
# -----------------------------------------------------------------------------
function Test-Setup {
    Write-Info 'Verification'
    $failCount = 0

    foreach ($cmd in 'cl', 'link', 'cmake', 'ninja', 'nasm', 'lld-link', 'clang', 'git', 'curl') {
        if (Get-Command $cmd -ErrorAction SilentlyContinue) {
            Write-Ok $cmd
        }
        else {
            Write-Fail "$cmd missing"
            $failCount++
        }
    }

    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Ok "rustc $(rustc --version)"
    }
    else {
        Write-Fail 'rustc missing'
        $failCount++
    }

    if (Get-Command bun -ErrorAction SilentlyContinue) {
        Write-Ok "bun $(bun --version)"
    }
    else {
        Write-Fail 'bun missing'
        $failCount++
    }

    $installedTargets = (rustup target list --installed) -join "`n"
    foreach ($tgt in 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu', 'wasm32-wasi', 'wasm32-unknown-unknown') {
        if ($installedTargets -match $tgt) {
            Write-Ok "target: $tgt"
        }
        else {
            Write-Fail "target missing: $tgt"
            $failCount++
        }
    }

    foreach ($tool in 'sccache', 'cargo-nextest', 'cargo-deny', 'cargo-vet', 'cargo-machete') {
        if (Get-Command $tool -ErrorAction SilentlyContinue) {
            Write-Ok $tool
        }
        else {
            Write-Fail "$tool missing"
            $failCount++
        }
    }

    if ($failCount -gt 0) {
        Write-Fail "$failCount component(s) missing"
        return $false
    }
    Write-Ok 'all components present'
    return $true
}

# -----------------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------------
Write-Info "aphrody — Windows setup ($Mode)"
Write-Info "repo: $RepoRoot"

switch ($Mode) {
    'install' {
        Install-WinGetCatalog
        Install-Bun
        Install-Rust
        Install-CargoTools
        Test-Setup | Out-Null
    }
    'Tools' {
        Install-CargoTools
        Test-Setup | Out-Null
    }
    'Check' {
        Test-Setup | Out-Null
    }
}

Write-Info 'Done. Build with: cargo build --release -p cli --target x86_64-pc-windows-msvc'
