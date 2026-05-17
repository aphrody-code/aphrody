$exe = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road\nie.exe'
$bytes = [System.IO.File]::ReadAllBytes($exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)

$patterns = @(
    'UnrealEngine', 'UnrealEd', 'UE4_', 'UE5_',
    'Unity', 'mono-', 'il2cpp',
    'CRIWARE', 'CRI_', 'criware', 'CpkMaker', 'sof_dec', 'criatomex',
    'criadx2', 'criha_', 'crimv',
    'Level5', 'Level-5', 'phyre', 'Phyre', 'YETI', 'Yeti',
    'denuvo', 'Denuvo', 'EOSSDK', 'EpicOnline', 'EasyAntiCheat',
    'inazuma', 'INAZUMA', 'Inazuma', 'nie_', 'NIE_',
    'D3D11', 'd3d12', 'Vulkan', 'OpenGL',
    'fmod', 'wwise', 'Wwise', 'WWISE',
    'Steamworks', 'SteamAPI',
    'react', 'flutter', 'cocos',
    'godot', 'cryengine'
)

foreach ($p in $patterns) {
    $idx = $ascii.IndexOf($p)
    $count = 0
    $samples = @()
    while ($idx -ge 0 -and $count -lt 5) {
        # Extract surrounding printable run
        $start = $idx
        while ($start -gt 0 -and $ascii[$start - 1] -ge ' ' -and [byte][char]$ascii[$start - 1] -le 126) { $start-- }
        $end = $idx + $p.Length
        while ($end -lt $ascii.Length -and $ascii[$end] -ge ' ' -and [byte][char]$ascii[$end] -le 126) { $end++ }
        $samples += $ascii.Substring($start, [Math]::Min(200, $end - $start))
        $count++
        $idx = $ascii.IndexOf($p, $idx + $p.Length)
    }
    if ($count -gt 0) {
        $totalHits = if ($idx -ge 0) { "$count+" } else { "$count" }
        Write-Output "--MATCH:$p ($totalHits hits)--"
        foreach ($s in $samples) { Write-Output "  $s" }
    }
}
