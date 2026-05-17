@echo off
:: SPDX-License-Identifier: Apache-2.0
:: =============================================================================
::  aphrody -- one-script dev-environment bootstrap (Windows cmd.exe)
::
::  Mission: lower contribution barrier to
::    git clone <repo> ^&^& scripts\dev-setup.cmd ^&^& cargo check
::
::  Native complement to .devcontainer\devcontainer.json -- installs the SAME
::  toolchain (rust nightly + wasm + bun + cargo extras) so a non-Codespace
::  contributor on Windows can match Codespace parity in one shot.
::
::  For deeper Win11 provisioning (winget VS 2026, SDK 26100, NASM, Ninja),
::  use scripts\setup-win.ps1 instead. This script is intentionally minimal
::  and assumes git + a C/C++ toolchain (MSVC or clang) are already present.
::
::  Properties:
::    - idempotent (safe to re-run after a partial setup)
::    - no admin / UAC required (uses winget user-scope + rustup-init)
::    - no whole-workspace build (only `cargo check -p aphrody`)
::
::  Usage:
::    scripts\dev-setup.cmd
:: =============================================================================
setlocal enableextensions enabledelayedexpansion

set "REPO_ROOT=%~dp0.."
pushd "%REPO_ROOT%" >nul

echo.
echo [aphrody] dev-setup -- Windows native
echo [aphrody] repo: %REPO_ROOT%
echo.

:: ---------- 1. bun (JS/TS runtime -- node forbidden by policy) --------------
echo [1/6] bun
where bun >nul 2>&1
if errorlevel 1 (
    echo   bun missing. Install via winget:
    echo     winget install --id Oven-sh.Bun --source winget --accept-source-agreements --accept-package-agreements
    echo   or PowerShell:
    echo     powershell -ExecutionPolicy Bypass -Command "irm bun.sh/install.ps1 ^| iex"
    echo   then re-open this shell and re-run dev-setup.cmd.
    goto :fail
)
for /f "tokens=*" %%v in ('bun --version 2^>nul') do set "BUN_VER=%%v"
echo   OK bun !BUN_VER!

:: ---------- 2. rustup (toolchain manager) -----------------------------------
echo [2/6] rustup
where rustup >nul 2>&1
if errorlevel 1 (
    echo   rustup missing. Install via winget:
    echo     winget install --id Rustlang.Rustup --source winget --accept-source-agreements --accept-package-agreements
    echo   or download: https://win.rustup.rs/x86_64
    echo   then re-open this shell and re-run dev-setup.cmd.
    goto :fail
)
echo   OK rustup present

:: ---------- 3. nightly toolchain + components -------------------------------
echo [3/6] rust nightly + components
call rustup install nightly
if errorlevel 1 goto :fail
call rustup default nightly
if errorlevel 1 goto :fail
call rustup component add rustfmt clippy rust-src rust-analyzer
if errorlevel 1 goto :fail
echo   OK nightly + components ready

:: ---------- 4. cross-compile targets (priorities 1, 2, 3) -------------------
echo [4/6] cross-compile targets
call rustup target add wasm32-unknown-unknown wasm32-wasip1 x86_64-unknown-linux-gnu
if errorlevel 1 goto :fail
echo   OK wasm32-unknown-unknown, wasm32-wasip1, x86_64-unknown-linux-gnu

:: ---------- 5. cargo extras (test runner + supply-chain + wasm) -------------
:: Match .devcontainer\devcontainer.json postCreateCommand exactly.
echo [5/6] cargo extras (may take a few minutes on first run)
for %%T in (cargo-nextest cargo-deny cargo-vet cargo-machete cargo-zigbuild cargo-auditable wasm-pack) do (
    call cargo install --list 2>nul | findstr /b /c:"%%T " >nul
    if errorlevel 1 (
        echo   installing %%T
        call cargo install --locked %%T
        if errorlevel 1 echo   WARN failed: %%T ^(continuing -- re-run later^)
    ) else (
        echo   OK %%T already installed
    )
)

:: ---------- 6. bun workspace install ----------------------------------------
echo [6/6] bun install ^(workspace deps^)
if exist "%REPO_ROOT%\bun.lock" (
    call bun install --frozen-lockfile
) else (
    echo   WARN no bun.lock -- running unlocked bun install
    call bun install
)
if errorlevel 1 goto :fail
echo   OK bun workspaces synced

:: ---------- verification ----------------------------------------------------
echo.
echo Verifying: cargo check -p aphrody --locked --offline
call cargo check -p aphrody --locked --offline
if errorlevel 1 (
    echo   WARN verification check returned non-zero ^(see output above^)
) else (
    echo   OK verification passed
)

:: ---------- success ---------------------------------------------------------
echo.
echo [aphrody] Dev environment ready. Try: cargo build --release -p aphrody
popd >nul
endlocal
exit /b 0

:fail
echo.
echo [aphrody] dev-setup FAILED -- see error above.
popd >nul
endlocal
exit /b 1
