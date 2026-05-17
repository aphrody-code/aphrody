$root = 'C:\Program Files (x86)\Steam\steamapps\common\INAZUMA ELEVEN Victory Road'
$cpk = Get-ChildItem -LiteralPath "$root\data\packs" -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -eq '.cpk' }
$stats = $cpk | Measure-Object -Property Length -Sum -Minimum -Maximum -Average
Write-Host "CPK_COUNT=$($cpk.Count)"
Write-Host "CPK_TOTAL_GB=$([math]::Round($stats.Sum/1GB,3))"
Write-Host "CPK_MIN_KB=$([math]::Round($stats.Minimum/1KB,2))"
Write-Host "CPK_MAX_MB=$([math]::Round($stats.Maximum/1MB,2))"
Write-Host "CPK_AVG_MB=$([math]::Round($stats.Average/1MB,2))"
Write-Host "---TOP20_LARGEST---"
$cpk | Sort-Object Length -Descending | Select-Object -First 20 |
    ForEach-Object { "$($_.Name)|$([math]::Round($_.Length/1MB,2))" }
Write-Host "---BOTTOM5_SMALLEST---"
$cpk | Sort-Object Length | Select-Object -First 5 |
    ForEach-Object { "$($_.Name)|$([math]::Round($_.Length/1KB,2))KB" }
Write-Host "---SIZE_HISTOGRAM---"
$buckets = @(
    @{ Label='<10KB';     Min=0;        Max=10KB }
    @{ Label='10KB-100KB';Min=10KB;     Max=100KB }
    @{ Label='100KB-1MB'; Min=100KB;    Max=1MB }
    @{ Label='1-10MB';    Min=1MB;      Max=10MB }
    @{ Label='10-100MB';  Min=10MB;     Max=100MB }
    @{ Label='100MB-1GB'; Min=100MB;    Max=1GB }
    @{ Label='>=1GB';     Min=1GB;      Max=[long]::MaxValue }
)
foreach ($b in $buckets) {
    $n = ($cpk | Where-Object { $_.Length -ge $b.Min -and $_.Length -lt $b.Max }).Count
    Write-Host "$($b.Label)|$n"
}
