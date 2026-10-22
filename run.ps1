# Avvia CineMatch con la build aggiornata.
# Uso: tasto destro -> Esegui con PowerShell, oppure  .\run.ps1
#
# Fa sempre, in quest'ordine:
#   1. chiude l'istanza aperta (se c'e), cosi l'exe si puo sostituire;
#   2. ricompila in release (incrementale: pochi secondi se il codice non e cambiato);
#   3. avvia il binario fresco. In fondo alle Impostazioni l'app mostra «build gg/mm hh:mm»:
#      se la data non e quella di adesso, stai guardando una finestra vecchia.
# L'updater e disattivato: questa e una build di sviluppo, non l'app installata.

$ErrorActionPreference = "Continue"
$root = $PSScriptRoot
$exe = Join-Path $root "target\release\cinematch.exe"

# Cargo e la toolchain GNU: binutils (dlltool) raggiungibile.
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }
$mingw = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
if (Test-Path $mingw) { $env:Path = "$mingw;$env:Path" }

$running = Get-Process cinematch -ErrorAction SilentlyContinue
if ($running) {
    $running | Stop-Process -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400
    Write-Host "Chiusa l'istanza precedente." -ForegroundColor DarkGray
}

Write-Host "Compilazione release (incrementale)..." -ForegroundColor Cyan
Push-Location $root
cargo build --release
$code = $LASTEXITCODE
Pop-Location
if ($code -ne 0) {
    Write-Host "Compilazione fallita: l'errore e scritto sopra." -ForegroundColor Red
    Read-Host "Premi Invio per chiudere"
    exit 1
}

$env:CINEMATCH_NO_UPDATE = "1"
Start-Process -FilePath $exe
$when = (Get-Date).ToString("dd/MM HH:mm")
Write-Host "CineMatch avviato: build $when. La trovi in fondo alle Impostazioni." -ForegroundColor Green
Write-Host "La chiave API TMDB si imposta dalla finestra Impostazioni (ingranaggio)." -ForegroundColor Green
