$root = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road'

function Show-Hex($path, $label, $bytes) {
    Write-Host "===$label==="
    Write-Host "PATH=$path"
    $fs = [System.IO.File]::OpenRead($path)
    try {
        $buf = New-Object byte[] $bytes
        $n = $fs.Read($buf, 0, $bytes)
        $hex = ($buf[0..($n-1)] | ForEach-Object { $_.ToString('x2') }) -join ' '
        $asc = ($buf[0..($n-1)] | ForEach-Object {
            if ($_ -ge 32 -and $_ -lt 127) { [char]$_ } else { '.' }
        }) -join ''
        Write-Host "HEX=$hex"
        Write-Host "ASC=$asc"
    } finally {
        $fs.Close()
    }
}

# Sample 1st cpk header (CRIWARE CPK has magic 'CPK ')
$firstCpk = Get-ChildItem -LiteralPath "$root\data\packs" -File | Select-Object -First 1
Show-Hex $firstCpk.FullName 'CPK_HEADER' 64

# USM header (CRIWARE USM = video)
Show-Hex "$root\data\dx11\movie\IE_15th.usm" 'USM_HEADER' 64

# Main exe header (PE) -- look for engine signatures
Show-Hex "$root\nie.exe" 'NIE_EXE_HEADER' 256

# cpk_list.cfg.bin (might be Phyre / Level5 catalog)
Show-Hex "$root\data\cpk_list.cfg.bin" 'CPK_LIST_HEADER' 128

# app_config
Show-Hex "$root\data\common\system\app_config_5.00.24.00.cfg.bin" 'APP_CONFIG_HEADER' 128
