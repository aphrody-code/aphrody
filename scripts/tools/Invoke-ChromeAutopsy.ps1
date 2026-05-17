[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$WorkspaceRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$ReverseEngineExe = "$WorkspaceRoot\src\Winclean.ReverseEngine\bin\Release\net10.0\win-x64\publish\Winclean.ReverseEngine.exe"

$ChromeDir = "C:\Program Files\Google\Chrome\Application"
$ChromeExe = "$ChromeDir\chrome.exe"
$ChromeDll = "$ChromeDir\134.0.6998.66\chrome.dll" # On prend chrome.exe par défaut s'il n'y a pas la dll sous la main
$Target = if (Test-Path $ChromeDll) { $ChromeDll } else { $ChromeExe }

$ReportPath = "$WorkspaceRoot\var\data\chrome_autopsy_report.json"

Write-Host "========================================="
Write-Host " AUTOPSIE EXTRÊME DE CHROME (WINCLEAN V2)"
Write-Host "========================================="
Write-Host "Cible : $Target"

if (-not (Test-Path $ReverseEngineExe)) {
    Write-Host "Compilation de Winclean.ReverseEngine V2 avec Iced..."
    Set-Location "$WorkspaceRoot\src\Winclean.ReverseEngine"
    dotnet publish -c Release -r win-x64
}

Write-Host "`n1. Lancement du NativeAOT Deep Scanner (PeNet + Iced)..."
$sw = [Diagnostics.Stopwatch]::StartNew()
& $ReverseEngineExe $Target > $ReportPath
$sw.Stop()
Write-Host "✅ Scan C# terminé en $($sw.Elapsed.TotalMilliseconds) ms."

Write-Host "`n2. Orchestration de la Toolchain LLVM (objdump/strings)..."
# Vérification de llvm-objdump dans le PATH (Si installé via vcpkg/llvm)
$llvmObjdump = Get-Command "llvm-objdump" -ErrorAction SilentlyContinue
if ($llvmObjdump) {
    Write-Host "✅ llvm-objdump détecté. Lancement de l'analyse headers..."
    & llvm-objdump -x $Target > "$WorkspaceRoot\var\data\chrome_llvm_headers.txt"
} else {
    Write-Host "⚠️ llvm-objdump non trouvé. Analyse LLVM sautée."
}

Write-Host "`nAutopsie terminée avec succès !"
Write-Host "Le rapport complet (ASM + Strings + PE Headers) est disponible ici :"
Write-Host "-> $ReportPath"
