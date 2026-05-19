# SPDX-License-Identifier: Apache-2.0
# Setup environment variables for aphrody dev work (Windows User scope, idempotent).
#
# Usage :
#   pwsh -File scripts/setup-dev-env.ps1            # apply with confirmations
#   pwsh -File scripts/setup-dev-env.ps1 -Apply     # apply silently
#   pwsh -File scripts/setup-dev-env.ps1 -Check     # dry-run (show plan only)
#   pwsh -File scripts/setup-dev-env.ps1 -Reset     # remove aphrody-specific vars (does not touch CARGO_HOME / RUSTUP_HOME)

param(
    [switch]$Apply,
    [switch]$Check,
    [switch]$Reset
)

# ---------------------------------------------------------------------------
# Canonical env table — single source of truth for the dev environment.
# ---------------------------------------------------------------------------
# Each entry : Name = canonical value. The script reads current User scope,
# compares, applies only if different. Reset removes the entry.
# ---------------------------------------------------------------------------
# Resolve user-specific paths via $env:USERPROFILE so the script is portable
# across machines (any Windows user, any home-dir layout). Falls back to
# `$HOME` on PowerShell-Core on non-Windows hosts.
$userHome = if ($env:USERPROFILE) { $env:USERPROFILE } else { $HOME }

$envTable = @{
    # Build cache + parallelism
    'CARGO_INCREMENTAL'                = '0'                                            # required for sccache
    'SCCACHE_DIR'                      = 'C:\sccache'                                   # cache dir (machine-wide, fast NVMe)
    'SCCACHE_CACHE_SIZE'               = '10G'                                          # bound size
    'CARGO_BUILD_JOBS'                 = '7'                                            # 8 cores - 1 linker

    # Cargo behaviour
    'CARGO_NET_GIT_FETCH_WITH_CLI'     = 'true'                                         # use git CLI (faster Win)
    'CARGO_TERM_COLOR'                 = 'always'                                       # color output in CI logs
    'RUST_BACKTRACE'                   = '1'                                            # backtraces by default

    # Bun caching (per-user, resolved via $env:USERPROFILE — see $userHome above)
    'BUN_RUNTIME_TRANSPILER_CACHE_PATH'= (Join-Path $userHome '.bun-transpile-cache')

    # Project-specific (aphrody)
    'APHRODY_A2A_ENDPOINT'             = 'http://localhost:8788/jsonrpc'                # A2A coord listener
    'APHRODY_LIVE_BACKEND'             = 'gemini-oauth'                                 # gemini-live-aphrody default
}

# ---------------------------------------------------------------------------
# Vars deliberately NOT touched (user-managed) :
#   CARGO_HOME, RUSTUP_HOME, BUN_INSTALL, GITHUB_PERSONAL_ACCESS_TOKEN
# If you want to reset CARGO_HOME / RUSTUP_HOME drift, do it manually.
# ---------------------------------------------------------------------------

function Plan-Diff {
    $rows = foreach ($name in $envTable.Keys | Sort-Object) {
        $expected = $envTable[$name]
        $current  = [Environment]::GetEnvironmentVariable($name, 'User')
        $verb     = if ($current -eq $expected) { 'OK' }
                    elseif ($current -eq $null) { 'SET' }
                    else { 'UPDATE' }
        [PSCustomObject]@{
            Name     = $name
            Current  = if ($current -eq $null) { '<unset>' } else { $current }
            Expected = $expected
            Action   = $verb
        }
    }
    $rows | Format-Table -AutoSize | Out-String | Write-Host
}

function Apply-Diff {
    foreach ($name in $envTable.Keys) {
        $expected = $envTable[$name]
        $current  = [Environment]::GetEnvironmentVariable($name, 'User')
        if ($current -ne $expected) {
            [Environment]::SetEnvironmentVariable($name, $expected, 'User')
            Write-Host ("APPLIED  {0} = {1}" -f $name, $expected)
        } else {
            Write-Host ("OK       {0}" -f $name)
        }
    }
    # Ensure cache dirs exist (sccache is machine-wide, bun-transpile-cache is per-user)
    foreach ($d in @('C:\sccache', (Join-Path $userHome '.bun-transpile-cache'))) {
        if (-not (Test-Path $d)) {
            New-Item -ItemType Directory -Force -Path $d | Out-Null
            Write-Host ("MKDIR    {0}" -f $d)
        }
    }
    Write-Host ''
    Write-Host 'Restart your shell to pick up the new User-scope env vars.'
}

function Reset-Vars {
    foreach ($name in $envTable.Keys) {
        $current = [Environment]::GetEnvironmentVariable($name, 'User')
        if ($current -ne $null) {
            [Environment]::SetEnvironmentVariable($name, $null, 'User')
            Write-Host ("REMOVED  {0}" -f $name)
        }
    }
}

# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------
if ($Reset) {
    Reset-Vars
    exit 0
}

if ($Check) {
    Write-Host 'Dry-run plan (no changes) :'
    Plan-Diff
    exit 0
}

if ($Apply) {
    Apply-Diff
    exit 0
}

# Default : show plan + ask
Write-Host 'Pending env changes (User scope) :'
Plan-Diff
$resp = Read-Host 'Apply ? [y/N]'
if ($resp -eq 'y' -or $resp -eq 'Y') {
    Apply-Diff
} else {
    Write-Host 'Aborted (no changes).'
}
