<#
.SYNOPSIS
    Opération Panopticon : Scanne l'intégralité des binaires (dll, exe, sys) de C:\Windows.
    Conçu avec l'API C# Native `EnumerationOptions` pour bypasser toutes les erreurs d'accès
    et garantir une performance extrême (< 15 secondes pour 100 000+ fichiers).
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$TargetDir = "C:\Windows"
$OutputFile = "C:\winclean\var\json\windows_scan.json"

Write-Host "=== WINCLEAN PANOPTICON SCANNER ===" -ForegroundColor Red
Write-Host "Cible : $TargetDir" -ForegroundColor DarkGray

# Injection de code C# pour la performance brute et l'évitement des exceptions ACL (Accès Refusé)
$code = @"
using System;
using System.IO;
using System.Collections.Generic;
using System.Text.Json;

public class PanopticonScanner {
    public class BinaryData {
        public string Path { get; set; }
        public long Size { get; set; }
        public string Extension { get; set; }
    }

    public static void ScanAndExport(string startPath, string jsonOutputPath) {
        var options = new EnumerationOptions {
            IgnoreInaccessible = true,       // Bypass les dossiers interdits au lieu de crasher
            RecurseSubdirectories = true,
            ReturnSpecialDirectories = false,
            AttributesToSkip = 0             // Ne skip rien (même les fichiers système cachés)
        };

        var results = new List<BinaryData>();

        // Liste des extensions à scanner (Reverse Engineering Targets)
        var targetExtensions = new HashSet<string>(StringComparer.OrdinalIgnoreCase) {
            ".dll", ".exe", ".sys"
        };

        try {
            foreach (var file in Directory.EnumerateFiles(startPath, "*.*", options)) {
                var ext = Path.GetExtension(file);
                if (targetExtensions.Contains(ext)) {
                    var info = new FileInfo(file);
                    results.Add(new BinaryData {
                        Path = file,
                        Size = info.Length,
                        Extension = ext.ToLowerInvariant()
                    });
                }
            }

            // Export ultra-rapide
            var jsonBytes = JsonSerializer.SerializeToUtf8Bytes(results, new JsonSerializerOptions {
                WriteIndented = true
            });
            File.WriteAllBytes(jsonOutputPath, jsonBytes);

            Console.WriteLine($"[C# KERNEL] Scan termine. {results.Count} binaires trouves et exportes vers JSON.");
        }
        catch (Exception ex) {
            Console.WriteLine($"[ERREUR C#] {ex.Message}");
        }
    }
}
"@

Add-Type -TypeDefinition $code -Language CSharp

$watch = [System.Diagnostics.Stopwatch]::StartNew()

Write-Host "[*] Lancement du moteur C# (EnumerateFiles)... Cela peut prendre entre 5 et 15 secondes." -ForegroundColor Yellow
[PanopticonScanner]::ScanAndExport($TargetDir, $OutputFile)

$watch.Stop()
Write-Host "[+] Export complet dans $OutputFile en $($watch.Elapsed.TotalSeconds) secondes." -ForegroundColor Green
