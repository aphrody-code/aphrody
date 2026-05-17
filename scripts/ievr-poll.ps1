$f = 'C:\Users\yohan\AppData\Local\Temp\ievr-strings.txt'
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
