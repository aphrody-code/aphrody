$root = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road'

# Multi-CPK header signature check (first 4 bytes, hex) — group by occurrences
$cpkDir = "$root\data\packs"
$cpkFiles = Get-ChildItem -LiteralPath $cpkDir -File | Where-Object { $_.Extension -eq '.cpk' }
$sigCount = @{}
$total = 0
foreach ($f in $cpkFiles) {
    $fs = [System.IO.File]::OpenRead($f.FullName)
    try {
        $buf = New-Object byte[] 4
        $null = $fs.Read($buf, 0, 4)
        $key = ($buf | ForEach-Object { $_.ToString('x2') }) -join ''
        if ($sigCount.ContainsKey($key)) { $sigCount[$key] += 1 } else { $sigCount[$key] = 1 }
        $total++
    } finally {
        $fs.Close()
    }
}
Write-Output "CPK_HEADER_DISTINCT=$($sigCount.Count)  TOTAL=$total"
Write-Output "TOP10_HEADER_BYTES:"
$sigCount.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 10 | ForEach-Object {
    Write-Output "  $($_.Name) -> $($_.Value)"
}

# Check overall: anything without extension?
$noext = Get-ChildItem -LiteralPath $root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { -not $_.Extension }
Write-Output "NOEXT_COUNT=$($noext.Count)"
foreach ($f in $noext | Select-Object -First 20) {
    Write-Output "  $($f.FullName.Substring($root.Length+1)) ($($f.Length) bytes)"
}

# Hidden / system files
$hidden = Get-ChildItem -LiteralPath $root -Recurse -File -Force -ErrorAction SilentlyContinue |
    Where-Object { ($_.Attributes -band [System.IO.FileAttributes]::Hidden) -or ($_.Attributes -band [System.IO.FileAttributes]::System) }
Write-Output "HIDDEN_COUNT=$($hidden.Count)"
foreach ($f in $hidden | Select-Object -First 20) {
    Write-Output "  $($f.FullName.Substring($root.Length+1)) [$($f.Attributes)]"
}
