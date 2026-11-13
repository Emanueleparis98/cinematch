param(
    [string]$Wheel = "",
    [string]$Python = "3.13",
    [switch]$NoPath
)
$ErrorActionPreference = "Stop"
$TermoviesTemp = Join-Path ([IO.Path]::GetTempPath()) ("termovies-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $TermoviesTemp | Out-Null
try {
    if (-not $Wheel) {
        $TermoviesBase = 'https://github.com/Emanueleparis98/termovies/releases'
        $TermoviesRelease = Invoke-RestMethod -Uri "$TermoviesBase/latest/download/latest.json"
        $TermoviesVersion = [string]$TermoviesRelease.version
        if ($TermoviesVersion -notmatch '^\d+\.\d+\.\d+$') { throw 'Versione della release non valida.' }
        $TermoviesWheelName = "termovies-$TermoviesVersion-py3-none-any.whl"
        $Wheel = Join-Path $TermoviesTemp $TermoviesWheelName
        Invoke-WebRequest -Uri "$TermoviesBase/download/terminal-v$TermoviesVersion/$TermoviesWheelName" -OutFile $Wheel
        Invoke-WebRequest -Uri "$TermoviesBase/download/terminal-v$TermoviesVersion/SHA256SUMS" -OutFile (Join-Path $TermoviesTemp 'SHA256SUMS')
    }
    $Wheel = (Resolve-Path -LiteralPath $Wheel).Path
    $TermoviesChecksums = Join-Path (Split-Path -Parent $Wheel) 'SHA256SUMS'
    if (-not (Test-Path -LiteralPath $TermoviesChecksums)) {
        $TermoviesChecksums = Join-Path (Split-Path -Parent $Wheel) 'SHA256SUMS-terminal.txt'
    }
    if (-not (Test-Path -LiteralPath $TermoviesChecksums)) { throw 'Manca SHA256SUMS accanto al pacchetto.' }
    $TermoviesName = [IO.Path]::GetFileName($Wheel)
    $TermoviesExpected = @((Get-Content -LiteralPath $TermoviesChecksums) | ForEach-Object {
        if ($_ -match '^([a-fA-F0-9]{64})\s+(.+)$' -and $Matches[2] -ceq $TermoviesName) { $Matches[1] }
    })
    # .NET works in Windows PowerShell 5.1 and PowerShell 7, without depending
    # on the optional/autoloaded Get-FileHash cmdlet module.
    $TermoviesHasher = [Security.Cryptography.SHA256]::Create()
    $TermoviesInputFile = [IO.File]::OpenRead($Wheel)
    try {
        $TermoviesActual = [BitConverter]::ToString($TermoviesHasher.ComputeHash($TermoviesInputFile)).Replace('-', '')
    } finally {
        $TermoviesInputFile.Dispose()
        $TermoviesHasher.Dispose()
    }
    if ($TermoviesExpected.Count -ne 1 -or $TermoviesActual -ine $TermoviesExpected[0]) { throw 'Checksum del pacchetto non valido; installazione annullata.' }
    $TermoviesUv = $env:TERMOVIES_UV
    if (-not $TermoviesUv) {
        $TermoviesUvInfo = Get-Command uv -ErrorAction SilentlyContinue
        if ($TermoviesUvInfo) { $TermoviesUv = $TermoviesUvInfo.Source }
    }
    if (-not $TermoviesUv) {
        $TermoviesUv = Join-Path $env:USERPROFILE '.local\bin\uv.exe'
        if (-not (Test-Path -LiteralPath $TermoviesUv)) {
            Write-Host 'Installo uv dal sito ufficiale Astral…'
            $TermoviesOldNoPath = $env:UV_NO_MODIFY_PATH
            try {
                $env:UV_NO_MODIFY_PATH = '1'
                Invoke-WebRequest -Uri 'https://astral.sh/uv/install.ps1' -OutFile (Join-Path $TermoviesTemp 'uv-install.ps1')
                & (Join-Path $TermoviesTemp 'uv-install.ps1')
                if (-not (Test-Path -LiteralPath $TermoviesUv)) { throw 'Installazione di uv non completata.' }
            } finally { $env:UV_NO_MODIFY_PATH = $TermoviesOldNoPath }
        }
    }
    & $TermoviesUv tool install --python $Python --force $Wheel
    if ($LASTEXITCODE -ne 0) { throw 'Installazione di Termovies non riuscita.' }
    if (-not $NoPath) {
        & $TermoviesUv tool update-shell
        if ($LASTEXITCODE -ne 0) { throw 'Aggiornamento PATH non riuscito.' }
    }
    $TermoviesBin = ((& $TermoviesUv tool dir --bin) -join [Environment]::NewLine).Trim()
    & (Join-Path $TermoviesBin 'termovies.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Verifica del comando termovies non riuscita.' }
    Write-Host 'Installazione completata. Avvia con: termovies'
    Write-Host 'Se il comando non viene riconosciuto, riapri il terminale per aggiornare PATH.'
} finally {
    Remove-Item -LiteralPath $TermoviesTemp -Recurse -Force -ErrorAction SilentlyContinue
}
