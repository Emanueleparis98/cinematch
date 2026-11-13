# CI-only check in a disposable Scoop installation on the Windows runner.
$ErrorActionPreference = 'Stop'
$TermoviesScoop = Join-Path $env:RUNNER_TEMP 'termovies-scoop-check'
$env:SCOOP = $TermoviesScoop
$TermoviesInstaller = Join-Path $env:RUNNER_TEMP 'install-scoop.ps1'
Invoke-WebRequest 'https://get.scoop.sh' -OutFile $TermoviesInstaller
& $TermoviesInstaller -ScoopDir $TermoviesScoop -RunAsAdmin
if ($LASTEXITCODE -ne 0) { throw 'Cannot install Scoop in CI' }
$TermoviesScoopCommand = Join-Path $TermoviesScoop 'shims/scoop.ps1'
& $TermoviesScoopCommand bucket add termovies https://github.com/Emanueleparis98/termovies
if ($LASTEXITCODE -ne 0) { throw 'Cannot add Termovies bucket' }
& $TermoviesScoopCommand install termovies/termovies
if ($LASTEXITCODE -ne 0) { throw 'Cannot install Termovies through Scoop' }
& (Join-Path $TermoviesScoop 'shims/termovies.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Scoop shim cannot launch Termovies' }
