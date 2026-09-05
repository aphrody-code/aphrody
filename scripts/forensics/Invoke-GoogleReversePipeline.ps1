<#
.SYNOPSIS
Pipeline dédiée à l'ingénierie inverse totale du répertoire Google.
Mappe les fichiers, extrait les archives, lance l'Ultimate Extreme Pipeline (Ghidra/Magika) et synthétise via Claude CLI.
#>
[CmdletBinding()]
param(
    [string]$TargetDir = (Join-Path $env:LOCALAPPDATA 'Google\Google'),
    [string]$OutputDir = "C:\winclean\var\markdown\Google_Reverse"
)

$ErrorActionPreference = 'Stop'

Write-Host "=====================================================" -ForegroundColor Cyan
Write-Host "    WINCLEAN - GOOGLE EXTREME REVERSE PIPELINE       " -ForegroundColor Cyan
Write-Host "=====================================================" -ForegroundColor Cyan

if (-not (Test-Path $TargetDir)) {
    Write-Error "Le répertoire cible $TargetDir n'existe pas."
    exit 1
}

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# 1. Mapping Complet du répertoire
Write-Host "`n[*] 1. Mapping récursif du répertoire..." -ForegroundColor Yellow
$MapFile = Join-Path $OutputDir "directory_map.json"
$allFiles = Get-ChildItem -Path $TargetDir -Recurse -File -Force

$fileInventory = $allFiles | Select-Object FullName, Length, Extension, CreationTime
# Utilisation de ConvertTo-Json avec Depth pour éviter les troncatures
$fileInventory | ConvertTo-Json -Depth 3 | Out-File $MapFile -Encoding UTF8
Write-Host "    [+] Mapping terminé : $($allFiles.Count) fichiers identifiés." -ForegroundColor Green


# 2. Inventaires du code de reverse (Libs et CLI)
Write-Host "`n[*] 2. Inventaire des cibles de Reverse Engineering..." -ForegroundColor Yellow
$reverseTargets = $allFiles | Where-Object { $_.Extension -match "\.(exe|dll|msix|sys|pak)$" }

$inventoryFile = Join-Path $OutputDir "reverse_inventory.txt"
$reverseTargets.FullName | Out-File $inventoryFile -Encoding UTF8

$libs = $reverseTargets | Where-Object { $_.Extension -match "\.(dll|sys)$" }
$clis = $reverseTargets | Where-Object { $_.Extension -eq ".exe" }
Write-Host "    [+] $($libs.Count) bibliothèques (DLL/SYS) trouvées." -ForegroundColor Green
Write-Host "    [+] $($clis.Count) exécutables (CLI/EXE) trouvés." -ForegroundColor Green


# 3. Extraction agressive (Tout extraire)
Write-Host "`n[*] 3. Extraction des conteneurs (MSIX)..." -ForegroundColor Yellow
$msixFiles = $reverseTargets | Where-Object { $_.Extension -eq ".msix" }
foreach ($msix in $msixFiles) {
    $msixExtractDir = Join-Path $OutputDir ($msix.Name + "_extracted")
    Write-Host "    [>] Extraction de $($msix.Name) vers $msixExtractDir"

    # MSIX est un format ZIP sous le capot, on peut utiliser Expand-Archive
    try {
        Expand-Archive -Path $msix.FullName -DestinationPath $msixExtractDir -Force
        Write-Host "    [+] Extraction réussie." -ForegroundColor Green
    } catch {
        Write-Warning "    [-] Échec de l'extraction ZIP native. Le fichier nécessite MakePri ou Appx."
    }
}


# 4. Lancement de l'Ultimate Pipeline ML (Ghidra, Magika, Ollama)
Write-Host "`n[*] 4. Exécution de la Pipeline Extreme WinClean (Deep Autopsy)..." -ForegroundColor Yellow
$pythonScript = "C:\winclean\src\Winclean.MlCore\winclean_ml\extreme_pipeline.py"
if (Test-Path $pythonScript) {
    Push-Location "C:\winclean\src\Winclean.MlCore"
    try {
        # Lance le pipeline asynchrone sur tout le répertoire Google
        uv run $pythonScript $TargetDir
        Write-Host "    [+] Pipeline ML terminée avec succès." -ForegroundColor Green
    } catch {
        Write-Warning "    [-] Erreur lors de l'exécution de la pipeline ML."
    }
    Pop-Location
} else {
    Write-Warning "    [-] extreme_pipeline.py introuvable."
}


# 5. Synthèse Claude CLI (Bypass Permissions)
Write-Host "`n[*] 5. Synthèse cognitive via Claude CLI..." -ForegroundColor Yellow
$reportsDir = "C:\winclean\var\reports"
$claudeOutput = Join-Path $OutputDir "Claude_Reverse_Synthesis.md"

$claudePrompt = @"
Tu es un expert en rétro-ingénierie WinClean.
Examine le contenu de $TargetDir (inventorié dans $MapFile) et tous les rapports générés dans $reportsDir.
Fais un bilan de sécurité complet :
1. Y a-t-il des librairies ou CLI Google suspects ?
2. Quels sont les comportements profonds révélés par le code C décompilé par Ghidra ?
3. Le MSIX est-il légitime ?
Génère ton rapport au format Markdown.
"@

try {
    # Appel de l'agent local Claude avec bypass total des permissions
    Write-Host "    [>] claude -p '...' --permission-mode bypassPermissions" -ForegroundColor DarkGray
    claude -p $claudePrompt --permission-mode bypassPermissions | Out-File $claudeOutput -Encoding UTF8
    Write-Host "    [+] Rapport d'audit Claude généré : $claudeOutput" -ForegroundColor Green
} catch {
    Write-Warning "    [-] Claude CLI n'est pas disponible dans l'environnement actuel ou a échoué."
}

Write-Host "`n=====================================================" -ForegroundColor Cyan
Write-Host " [V] PIPELINE DÉDIÉE TERMINÉE " -ForegroundColor Cyan
Write-Host "=====================================================" -ForegroundColor Cyan
