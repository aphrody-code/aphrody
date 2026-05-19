# SPDX-License-Identifier: Apache-2.0
# Wait for the IEVR strings dump to appear in $env:TEMP and emit it.
# Uses $env:TEMP so the script is portable across users/machines (the
# previous version hard-coded `C:\Users\yohan\AppData\Local\Temp\...`).
$tempDir = if ($env:TEMP) { $env:TEMP } else { [System.IO.Path]::GetTempPath() }
$f = Join-Path $tempDir 'ievr-strings.txt'

for ($i = 0; $i -lt 90; $i++) {
    if (Test-Path $f) {
        $sz = (Get-Item $f).Length
        if ($sz -gt 100) {
            Get-Content $f
            return
        }
    }
    Start-Sleep -Seconds 2
}
Write-Host "TIMEOUT"
