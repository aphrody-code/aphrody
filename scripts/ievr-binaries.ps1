$root = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road'
$all = Get-ChildItem -LiteralPath $root -Recurse -File -ErrorAction SilentlyContinue

function Get-Sha256Short($path) {
    $out = & certutil -hashfile $path SHA256 2>$null
    foreach ($line in $out) {
        $t = $line.Trim()
        if ($t.Length -ge 32 -and $t -notmatch '[: ]' -and $t -match '^[0-9a-fA-F]+$') {
            return $t.Substring(0, 16).ToLower()
        }
    }
    return '[hash-failed]'
}

function Show-Group($ext, $hash) {
    Write-Host "---$ext---"
    $files = $all | Where-Object { $_.Extension.ToLower() -eq $ext }
    foreach ($f in $files | Sort-Object FullName) {
        $rel = $f.FullName.Substring($root.Length + 1)
        $sizeMB = [math]::Round($f.Length / 1MB, 3)
        if ($hash) {
            $h = Get-Sha256Short $f.FullName
        } else {
            $h = ''
        }
        Write-Host "$rel|$sizeMB|$h"
    }
}

Show-Group '.exe' $true
Show-Group '.dll' $true
Show-Group '.bin' $true
Show-Group '.usm' $true
Show-Group '.cer' $true
Show-Group '.vdf' $false
Show-Group '.ini' $false
Show-Group '.cfg' $false
Show-Group '.json' $false
Show-Group '.conf' $false
Show-Group '.png' $false
Show-Group '.txt' $false
Show-Group '.ps1' $false
Show-Group '.bat' $false
