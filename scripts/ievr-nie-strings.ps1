$exe = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road\nie.exe'
$bytes = [System.IO.File]::ReadAllBytes($exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)

# Strings of length >=6 made of printable chars
$re = [regex]'[\x20-\x7e]{6,}'
$matches = $re.Matches($ascii)
Write-Host "TOTAL_STRINGS=$($matches.Count)"

# Look for engine / framework / DRM / Level-5 markers
$patterns = @(
    'UnrealEngine', 'UnrealEd', 'UE4', 'UE5',
    'Unity', 'mono-', 'il2cpp',
    'CRIWARE', 'CRI ', 'criware', 'CpkMaker', 'sof_dec', 'criatomex',
    'Level5', 'Level-5', 'level5', 'level-5', 'phyre', 'YETI', 'Yeti',
    'denuvo', 'Denuvo', 'EOSSDK', 'EpicOnline', 'EasyAntiCheat',
    'inazuma', 'INAZUMA', 'Inazuma', 'NIE',
    'D3D11', 'd3d12', 'Vulkan', 'OpenGL', 'wgpu',
    'fmod', 'wwise', 'Wwise', 'WWISE',
    'Steamworks', 'SteamAPI'
)

foreach ($p in $patterns) {
    $hits = $matches | Where-Object { $_.Value -match [regex]::Escape($p) } | Select-Object -First 3
    if ($hits) {
        Write-Host "--MATCH:$p--"
        foreach ($h in $hits) { Write-Host "  $($h.Value)" }
    }
}
