$root = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road'
$all = Get-ChildItem -LiteralPath $root -Recurse -File -ErrorAction SilentlyContinue
Write-Host "TOTAL_FILES=$($all.Count)"
$sum = ($all | Measure-Object -Property Length -Sum).Sum
Write-Host "TOTAL_BYTES=$sum"
Write-Host "TOTAL_MB=$([math]::Round($sum/1MB,2))"
Write-Host "TOTAL_GB=$([math]::Round($sum/1GB,3))"
Write-Host "---EXT_GROUPS---"
$all | Group-Object { if ($_.Extension) { $_.Extension.ToLower() } else { '<noext>' } } |
    Sort-Object Count -Descending |
    Select-Object Count, @{N='Ext';E={$_.Name}}, @{N='TotalMB';E={[math]::Round((($_.Group | Measure-Object Length -Sum).Sum)/1MB, 2)}} |
    Format-Table -AutoSize | Out-String -Width 200
