[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$WorkspaceRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$ProjectDir = "$WorkspaceRoot\src\Winclean.ChromeDecryptor"
$BackupDir = "$WorkspaceRoot\var\data\chrome_backup"

Write-Host "Recompilation de Winclean.ChromeDecryptor en NativeAOT (Speed Optimized)..."
Set-Location $ProjectDir
dotnet publish -c Release -r win-x64 /p:IlcOptimizationPreference=Speed

$ExePath = "$ProjectDir\bin\Release\net10.0\win-x64\publish\Winclean.ChromeDecryptor.exe"

if (-not (Test-Path $ExePath)) {
    throw "L'exécutable compilé est introuvable."
}

Write-Host "`nLancement du Benchmark (10 itérations)..."

$TotalMilliseconds = 0

for ($i = 1; $i -le 10; $i++) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $output = & $ExePath $BackupDir
    $sw.Stop()

    $TotalMilliseconds += $sw.Elapsed.TotalMilliseconds
    Write-Host "Itération $i : $($sw.Elapsed.TotalMilliseconds.ToString('0.00')) ms"
}

$Avg = $TotalMilliseconds / 10
Write-Host "-----------------------------------------"
Write-Host "Moyenne d'exécution : $($Avg.ToString('0.00')) ms"
Write-Host "-----------------------------------------"

Set-Location $WorkspaceRoot
