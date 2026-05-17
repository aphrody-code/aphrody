#requires -Version 7
<#
.SYNOPSIS
    Light 5-point UI gate for the IEVR /ievr page. Hits gates 1+2 only.

.DESCRIPTION
    Runs the cheap two-thirds of the gate documented in winclean's CLAUDE.md
    delivery checklist:
      Gate 1 — all critical URLs return 200 (curl HEAD).
      Gate 2 — Edge headless screenshot + browser/JS console log capture.
    Gates 3-5 (WebGPU adapter, HUD live values, interaction) need CDP-driven
    automation (Playwright or chromedp). Out of scope here; bxc on the winclean
    side has DOM-only mirror (no GPU). Use `bxc mirror` for DOM snapshot and
    a real Playwright suite for the WebGPU gates.

    Result is printed as FAIT / INCOMPLET / NON_FAIT per the
    `aphrody honest-delivery/v1` extension contract.

.PARAMETER BaseUrl
    Default http://localhost:8787

.PARAMETER OutDir
    Where to drop screenshot + log. Default var/logs/ievr-verify/<timestamp>.

.EXAMPLE
    pwsh scripts/ievr-verify.ps1

.EXAMPLE
    pwsh scripts/ievr-verify.ps1 -BaseUrl http://192.168.1.50:8787
#>
param(
    [string]$BaseUrl = 'http://localhost:8787',
    [string]$OutDir
)

$ErrorActionPreference = 'Continue'

if (-not $OutDir) {
    $stamp = Get-Date -Format 'yyyyMMddTHHmmss'
    $OutDir = Join-Path $PSScriptRoot "..\var\logs\ievr-verify\$stamp"
}
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# ── Gate 1: HTTP 200 on critical URLs ────────────────────────────────────────
$urls = @(
    "$BaseUrl/ievr",
    "$BaseUrl/ievr.bootstrap.mjs",
    "$BaseUrl/wasm/ievr-engine/ievr_engine_bg.wasm",
    "$BaseUrl/wasm/ievr-engine/ievr_engine.js",
    "$BaseUrl/api/chars/stats",
    "$BaseUrl/api/gpu",
    "$BaseUrl/api/cpk/status"
)

$gate1 = @()
foreach ($u in $urls) {
    try {
        $r = Invoke-WebRequest -Uri $u -Method Head -TimeoutSec 5 -SkipHttpErrorCheck
        $gate1 += [pscustomobject]@{
            url = $u; status = $r.StatusCode; ok = ($r.StatusCode -eq 200)
        }
    } catch {
        $gate1 += [pscustomobject]@{
            url = $u; status = 'ERR'; ok = $false; error = $_.Exception.Message
        }
    }
}

$gate1_ok = ($gate1 | Where-Object { -not $_.ok }).Count -eq 0
$gate1 | Format-Table -AutoSize

# ── Gate 2: Edge headless screenshot + log capture ───────────────────────────
$edge = @(
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

$gate2_ok = $false
if ($edge) {
    $shot = Join-Path $OutDir 'ievr.png'
    $logf = Join-Path $OutDir 'edge.log'
    # virtual-time-budget lets the page run for 10s of simulated time before
    # the screenshot snapshot, giving WebGPU adapter request + first frames
    # the chance to land. Without this, headless screenshot fires before the
    # async navigator.gpu.requestAdapter() resolves.
    & $edge `
        --headless=new `
        --enable-features=Vulkan,WebGPU `
        --enable-unsafe-webgpu `
        --enable-logging --v=1 `
        --log-file="$logf" `
        --screenshot="$shot" `
        --window-size=1280,720 `
        --disable-gpu-sandbox `
        --virtual-time-budget=10000 `
        "$BaseUrl/ievr" 2>&1 | Out-Null

    Start-Sleep -Seconds 1
    $gate2_ok = (Test-Path $shot) -and ((Get-Item $shot).Length -gt 1024)
    if ($gate2_ok) {
        Write-Host "screenshot: $shot ($((Get-Item $shot).Length) bytes)" -ForegroundColor Green
    }
}

# ── Honest tri-state report ──────────────────────────────────────────────────
Write-Host ""
Write-Host "── IEVR verification report ──" -ForegroundColor Cyan
Write-Host "Gate 1 (HTTP 200): " -NoNewline
if ($gate1_ok) { Write-Host "FAIT" -ForegroundColor Green } else { Write-Host "INCOMPLET" -ForegroundColor Yellow }
Write-Host "Gate 2 (Edge screenshot taken): " -NoNewline
if ($gate2_ok) { Write-Host "FAIT" -ForegroundColor Green } else { Write-Host "INCOMPLET" -ForegroundColor Yellow }
Write-Host "Gate 3 (WebGPU adapter init): NON_FAIT (needs CDP-driven verification)" -ForegroundColor DarkGray
Write-Host "Gate 4 (HUD FPS != 0):        NON_FAIT (needs CDP eval)"               -ForegroundColor DarkGray
Write-Host "Gate 5 (interaction tested):  NON_FAIT (needs CDP click)"              -ForegroundColor DarkGray
Write-Host ""
Write-Host "Artefacts at: $OutDir" -ForegroundColor DarkGray
if (-not $gate1_ok -or -not $gate2_ok) { exit 1 }
exit 0
