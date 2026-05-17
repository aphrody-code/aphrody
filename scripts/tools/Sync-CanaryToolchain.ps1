<#
.SYNOPSIS
    Syncs the environment with the latest Google Canary toolchain.
.DESCRIPTION
    This script enforces the "Canary Channel Policy" defined in google-os-directives.md.
    It uses WinGet to locate and install/update all available Google Canary packages
    (Chrome Canary, Android Studio Canary) and dynamically updates google.json.
.EXAMPLE
    .\Sync-CanaryToolchain.ps1
#>

$ErrorActionPreference = 'Stop'

# Define our target Canary packages
$CanaryPackages = @(
    @{ Id = "Google.Chrome.Canary"; Name = "Google Chrome Canary" },
    @{ Id = "Google.AndroidStudio.Canary"; Name = "Android Studio Canary" }
)

Write-Host "[*] Enforcing Canary Channel Policy..." -ForegroundColor Cyan

# 1. Fast-track direct installation for Chrome Canary (Absolute latest from Google CDN)
$CanaryUrl = "https://dl.google.com/tag/s/appguid%3D%7B4EA16AC7-FD5A-47C3-875B-DBF4A2008C20%7D%26iid%3D%7B11C81182-0AB1-173D-16AC-4CEC14321819%7D%26lang%3Dfr%26browser%3D4%26usagestats%3D1%26appname%3DGoogle%2520Chrome%2520Canary%26needsadmin%3Dfalse%26ap%3D-arch_x64-statsdef_1%26installdataindex%3Dempty/update2/installers/ChromeSetup.exe"
$InstallerPath = Join-Path $env:TEMP "ChromeCanarySetup.exe"

Write-Host "[*] Downloading absolute latest Chrome Canary installer..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri $CanaryUrl -OutFile $InstallerPath -UseBasicParsing
    Write-Host "[*] Executing Chrome Canary installer..." -ForegroundColor Yellow
    $process = Start-Process -FilePath $InstallerPath -Wait -PassThru
    Write-Host "[+] Chrome Canary installation triggered." -ForegroundColor Green
} catch {
    Write-Host "[-] Failed to download/install from direct link: $_" -ForegroundColor Red
}

# 2. Sync remaining packages via WinGet
foreach ($Pkg in $CanaryPackages) {
    Write-Host "[*] Checking $($Pkg.Name) ($($Pkg.Id))..." -ForegroundColor Yellow

    # Run winget install/upgrade
    # Using --accept-package-agreements and --accept-source-agreements for silent execution
    $wingetArgs = @("install", "--id", $Pkg.Id, "--exact", "--silent", "--accept-package-agreements", "--accept-source-agreements")

    try {
        $process = Start-Process winget -ArgumentList $wingetArgs -Wait -NoNewWindow -PassThru
        if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 2316632065) { # 2316632065 = already installed
            Write-Host "[+] $($Pkg.Name) is up to date." -ForegroundColor Green
        } else {
            Write-Host "[-] WinGet returned exit code $($process.ExitCode) for $($Pkg.Id)." -ForegroundColor Red
        }
    } catch {
        Write-Host "[-] Failed to execute winget for $($Pkg.Id): $_" -ForegroundColor Red
    }
}

# Now we extract the installed versions to update google.json (optional but maintainable)
$GoogleJsonPath = Join-Path $PSScriptRoot "..\..\google.json"
if (Test-Path $GoogleJsonPath) {
    Write-Host "[*] Updating google.json with latest Canary versions..." -ForegroundColor Yellow
    $json = Get-Content $GoogleJsonPath -Raw | ConvertFrom-Json -Depth 10

    # Quick and dirty parser for winget list
    foreach ($Pkg in $CanaryPackages) {
        $installedInfo = winget list --id $Pkg.Id --exact | Select-String $Pkg.Id
        if ($installedInfo) {
            # Usually format is: Name Id Version Available Source
            # We just need the version regex, e.g., 150.0.7839.0
            if ($installedInfo -match "$($Pkg.Id)\s+(v?\d+\.\d+\.\d+\.\d+|\d+\.\d+\.\d+\.\d+)") {
                $version = $Matches[1]
                Write-Host "[+] Found $($Pkg.Id) version: $version" -ForegroundColor Green

                # Update it in the json if it exists in google_available_optional or google_installed
                foreach ($item in $json.winget_packages.google_available_optional) {
                    if ($item.id -eq $Pkg.Id) {
                        $item.latest = $version
                        $item.status = "installed"
                    }
                }
            }
        }
    }

    $json | ConvertTo-Json -Depth 10 | Set-Content $GoogleJsonPath -Encoding UTF8
    Write-Host "[+] google.json updated." -ForegroundColor Green
} else {
    Write-Host "[-] google.json not found at $GoogleJsonPath" -ForegroundColor Yellow
}

Write-Host "[*] Canary Sync Complete." -ForegroundColor Cyan
