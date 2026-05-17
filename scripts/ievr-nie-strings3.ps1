$exe = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road\nie.exe'
$bytes = [System.IO.File]::ReadAllBytes($exe)
$ascii = [System.Text.Encoding]::ASCII.GetString($bytes)

$patterns = @(
    'lua', 'LUA', 'LuaJIT', 'wren',
    'sqlite', 'SQLite',
    'icu', 'ICU ',
    'protobuf', 'protoc',
    'flatbuffer',
    'zlib', 'lz4', 'zstd', 'snappy',
    'imgui', 'ImGui',
    'curl', 'libcurl',
    'mbedtls', 'openssl',
    'YAGE', 'yage',
    'NieGame', 'nieGame', 'nie_game',
    'INAZUMA ELEVEN',
    'gameflow.txt', 'GAMEFLOW',
    'criatomex',
    'tcp', 'TCP', 'http_',
    'shader', 'Shader',
    'SHIPPING', 'Shipping', 'shipping',
    'DEBUG_BUILD', 'BUILD_'
)

foreach ($p in $patterns) {
    $idx = $ascii.IndexOf($p)
    $count = 0
    $samples = @()
    while ($idx -ge 0 -and $count -lt 3) {
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
        $samples += $ascii.Substring($start, [Math]::Min(120, $end - $start))
        $count++
        $idx = $ascii.IndexOf($p, $idx + $p.Length)
    }
    if ($count -gt 0) {
        Write-Output "--MATCH:$p--"
        foreach ($s in $samples) { Write-Output "  $s" }
    }
}

# Also dump PE machine + bitness from header
$peOff = [BitConverter]::ToInt32($bytes, 0x3C)
$sigBytes = $bytes[$peOff..($peOff+3)]
$machine = [BitConverter]::ToUInt16($bytes, $peOff + 4)
$nSections = [BitConverter]::ToUInt16($bytes, $peOff + 6)
$timestamp = [BitConverter]::ToInt32($bytes, $peOff + 8)
$ts = [DateTimeOffset]::FromUnixTimeSeconds($timestamp).ToString('u')
Write-Output "--PE_INFO--"
Write-Output "  PE offset: 0x$($peOff.ToString('X'))"
Write-Output "  Machine:   0x$($machine.ToString('X4'))  (0x8664=x64, 0x14C=x86, 0xAA64=arm64)"
Write-Output "  Sections:  $nSections"
Write-Output "  Timestamp: $ts (epoch $timestamp)"
