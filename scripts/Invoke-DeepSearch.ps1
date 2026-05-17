<#
.SYNOPSIS
    Deep Search Extrême : Requête l'index Windows (WSearch) via OLE DB / COM.
    Performances fulgurantes pour rechercher des fichiers sur l'ensemble du système (SQL-like).
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$Query,

    [int]$Limit = 100
)

$ErrorActionPreference = 'Stop'

# Formatage de la requête SQL pour le moteur de recherche Windows (CollatorDSO)
# Utilise la syntaxe FREETEXT ou CONTAINS selon le motif
$sqlQuery = "SELECT TOP $Limit System.ItemPathDisplay, System.ItemName, System.DateModified FROM SystemIndex WHERE CONTAINS('""*$Query*""')"

$connectionString = "Provider=Search.CollatorDSO;Extended Properties='Application=Windows';"

try {
    $connection = New-Object System.Data.OleDb.OleDbConnection($connectionString)
    $command = New-Object System.Data.OleDb.OleDbCommand($sqlQuery, $connection)
    $adapter = New-Object System.Data.OleDb.OleDbDataAdapter($command)
    $dataTable = New-Object System.Data.DataTable

    $watch = [System.Diagnostics.Stopwatch]::StartNew()

    $connection.Open()
    $adapter.Fill($dataTable) | Out-Null

    $watch.Stop()

    Write-Host "`n[WINCLEAN DEEP SEARCH]" -ForegroundColor Cyan
    Write-Host "Requete : $sqlQuery" -ForegroundColor DarkGray
    Write-Host "Temps d'execution : $($watch.ElapsedMilliseconds) ms" -ForegroundColor Green
    Write-Host "Resultats trouves : $($dataTable.Rows.Count)" -ForegroundColor Yellow
    Write-Host "----------------------------------------"

    if ($dataTable.Rows.Count -gt 0) {
        $dataTable | Format-Table -Property System.ItemPathDisplay, System.DateModified -AutoSize
    } else {
        Write-Host "Aucun resultat trouve dans l'index." -ForegroundColor Red
    }

} catch {
    Write-Error "Echec de l'execution OLE DB : $_"
} finally {
    if ($connection.State -eq 'Open') {
        $connection.Close()
    }
}
