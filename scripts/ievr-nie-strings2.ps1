$exe = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road\nie.exe'
$bytes = [System.IO.File]::ReadAllBytes($exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)

$patterns = @(
    'L5_', '_L5', 'l5_', 'LV5', 'Level5', 'level5',
    'gameflow', 'GameFlow',
    'Phyre', 'phyre',
    'engine_', 'Engine_',
    'CpkMaker',
    'sof_dec', 'criatomex', 'criadx', 'crimv',
    'fmodstudio', 'FMOD Studio',
    'YETI',
    'Vulkan', 'OpenGL', 'OpenGLES',
    'eos_', 'EOS_',
    'WebGPU', 'webgpu', 'wgpu'
)

foreach ($p in $patterns) {
    $idx = $ascii.IndexOf($p)
    $count = 0
    $samples = @()
    while ($idx -ge 0 -and $count -lt 5) {
        $start = $idx
        while ($start -gt 0) {
            $c = [byte][char]$ascii[$start - 1]
            if ($c -lt 32 -or $c -gt 126) { break }
            $start--
        }
        $end = $idx + $p.Length
        while ($end -lt $ascii.Length) {
            $c = [byte][char]$ascii[$end]
            if ($c -lt 32 -or $c -gt 126) { break }
            $end++
        }
        $samples += $ascii.Substring($start, [Math]::Min(160, $end - $start))
        $count++
        $idx = $ascii.IndexOf($p, $idx + $p.Length)
    }
    if ($count -gt 0) {
        Write-Output "--MATCH:$p--"
        foreach ($s in $samples) { Write-Output "  $s" }
    }
}
