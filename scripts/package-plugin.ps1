param([string]$Version = '1.0.0')
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Versione del plugin non valida: usa x.y.z' }
$taskWorkspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$taskPlugin = Join-Path $taskWorkspace 'plugins\cinema-currents'
$taskBinary = Join-Path $taskWorkspace 'target\release\cinema-currents.exe'
if (-not (Test-Path -LiteralPath $taskBinary)) { throw 'Compila prima: cargo build --release --workspace' }
$taskTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$taskStage = [IO.Path]::GetFullPath((Join-Path $taskTempRoot ('CineMatch-Currents-' + [guid]::NewGuid())))
if (-not $taskStage.StartsWith($taskTempRoot,[StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid staging path' }
New-Item -ItemType Directory -Path $taskStage | Out-Null
try {
    Copy-Item -LiteralPath $taskBinary -Destination $taskStage
    Copy-Item -LiteralPath (Join-Path $taskPlugin 'README.md') -Destination $taskStage
    Copy-Item -LiteralPath (Join-Path $taskPlugin 'request.example.json') -Destination $taskStage
    $taskSource = Join-Path $taskStage 'source'
    New-Item -ItemType Directory -Path $taskSource | Out-Null
    foreach ($taskName in @('Cargo.toml','README.md','request.example.json','src','assets','tests')) {
        Copy-Item -LiteralPath (Join-Path $taskPlugin $taskName) -Destination $taskSource -Recurse
    }
    $taskDist = Join-Path $taskWorkspace 'dist'
    New-Item -ItemType Directory -Path $taskDist -Force | Out-Null
    $taskZip = Join-Path $taskDist ('Cinema-Currents-' + $Version + '-windows-x64.zip')
    Compress-Archive -Path (Join-Path $taskStage '*') -DestinationPath $taskZip -Force
    Get-FileHash -LiteralPath $taskZip -Algorithm SHA256
} finally {
    $taskResolved = [IO.Path]::GetFullPath($taskStage)
    if (-not $taskResolved.StartsWith($taskTempRoot,[StringComparison]::OrdinalIgnoreCase) -or
        (Get-Item -LiteralPath $taskResolved).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Unsafe staging cleanup path' }
    if (Get-ChildItem -LiteralPath $taskResolved -Recurse -Force | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw 'Staging contains reparse points' }
    Remove-Item -LiteralPath $taskResolved -Recurse -Force
}
