param(
    [string]$Wheel = "",
    [string]$Python = "3.13",
    [switch]$NoPath
)
$ErrorActionPreference = "Stop"
$CinematchTemp = Join-Path ([IO.Path]::GetTempPath()) ("cinematch-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $CinematchTemp | Out-Null
try {
    if (-not $Wheel) {
        $CinematchBase = 'https://github.com/Emanueleparis98/cinematch/releases'
        $CinematchRelease = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/Emanueleparis98/cinematch/main/terminal/latest.json"
        $CinematchVersion = [string]$CinematchRelease.version
        if ($CinematchVersion -notmatch '^\d+\.\d+\.\d+$') { throw 'Versione della release non valida.' }
        $CinematchWheelName = "cinematch-$CinematchVersion-py3-none-any.whl"
        $Wheel = Join-Path $CinematchTemp $CinematchWheelName
        Invoke-WebRequest -Uri "$CinematchBase/download/terminal-v$CinematchVersion/$CinematchWheelName" -OutFile $Wheel
        Invoke-WebRequest -Uri "$CinematchBase/download/terminal-v$CinematchVersion/SHA256SUMS-terminal.txt" -OutFile (Join-Path $CinematchTemp 'SHA256SUMS-terminal.txt')
    }
    $Wheel = (Resolve-Path -LiteralPath $Wheel).Path
    $CinematchChecksums = Join-Path (Split-Path -Parent $Wheel) 'SHA256SUMS-terminal.txt'
    if (-not (Test-Path -LiteralPath $CinematchChecksums)) { throw 'Manca SHA256SUMS-terminal.txt accanto al pacchetto.' }
    $CinematchName = [IO.Path]::GetFileName($Wheel)
    $CinematchExpected = @((Get-Content -LiteralPath $CinematchChecksums) | ForEach-Object {
        if ($_ -match '^([a-fA-F0-9]{64})\s+(.+)$' -and $Matches[2] -ceq $CinematchName) { $Matches[1] }
    })
    # .NET works in Windows PowerShell 5.1 and PowerShell 7, without depending
    # on the optional/autoloaded Get-FileHash cmdlet module.
    $CinematchHasher = [Security.Cryptography.SHA256]::Create()
    $CinematchInputFile = [IO.File]::OpenRead($Wheel)
    try {
        $CinematchActual = [BitConverter]::ToString($CinematchHasher.ComputeHash($CinematchInputFile)).Replace('-', '')
    } finally {
        $CinematchInputFile.Dispose()
        $CinematchHasher.Dispose()
    }
    if ($CinematchExpected.Count -ne 1 -or $CinematchActual -ine $CinematchExpected[0]) { throw 'Checksum del pacchetto non valido; installazione annullata.' }
    $CinematchUv = $env:CINEMATCH_UV
    if (-not $CinematchUv) {
        $CinematchUvInfo = Get-Command uv -ErrorAction SilentlyContinue
        if ($CinematchUvInfo) { $CinematchUv = $CinematchUvInfo.Source }
    }
    if (-not $CinematchUv) {
        $CinematchUv = Join-Path $env:USERPROFILE '.local\bin\uv.exe'
        if (-not (Test-Path -LiteralPath $CinematchUv)) {
            Write-Host 'Installo uv dal sito ufficiale Astral…'
            $CinematchOldNoPath = $env:UV_NO_MODIFY_PATH
            try {
                $env:UV_NO_MODIFY_PATH = '1'
                Invoke-WebRequest -Uri 'https://astral.sh/uv/install.ps1' -OutFile (Join-Path $CinematchTemp 'uv-install.ps1')
                & (Join-Path $CinematchTemp 'uv-install.ps1')
                if (-not (Test-Path -LiteralPath $CinematchUv)) { throw 'Installazione di uv non completata.' }
            } finally { $env:UV_NO_MODIFY_PATH = $CinematchOldNoPath }
        }
    }
    & $CinematchUv tool install --python $Python --force $Wheel
    if ($LASTEXITCODE -ne 0) { throw 'Installazione di CineMatch non riuscita.' }
    if (-not $NoPath) {
        & $CinematchUv tool update-shell
        if ($LASTEXITCODE -ne 0) { throw 'Aggiornamento PATH non riuscito.' }
    }
    $CinematchBin = ((& $CinematchUv tool dir --bin) -join [Environment]::NewLine).Trim()
    & (Join-Path $CinematchBin 'cinematch.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Verifica del comando cinematch non riuscita.' }
    Write-Host 'Installazione completata. Avvia con: cinematch'
    Write-Host 'Se il comando non viene riconosciuto, riapri il terminale per aggiornare PATH.'
} finally {
    Remove-Item -LiteralPath $CinematchTemp -Recurse -Force -ErrorAction SilentlyContinue
}
