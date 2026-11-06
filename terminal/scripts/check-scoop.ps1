# CI-only check in a disposable Scoop installation on the Windows runner.
$ErrorActionPreference = 'Stop'
$CinematchScoop = Join-Path $env:RUNNER_TEMP 'cinematch-scoop-check'
$env:SCOOP = $CinematchScoop
$CinematchInstaller = Join-Path $env:RUNNER_TEMP 'install-scoop.ps1'
Invoke-WebRequest 'https://get.scoop.sh' -OutFile $CinematchInstaller
& $CinematchInstaller -ScoopDir $CinematchScoop -RunAsAdmin
if ($LASTEXITCODE -ne 0) { throw 'Cannot install Scoop in CI' }
$CinematchScoopCommand = Join-Path $CinematchScoop 'shims/scoop.ps1'
& $CinematchScoopCommand bucket add cinematch https://github.com/Emanueleparis98/cinematch
if ($LASTEXITCODE -ne 0) { throw 'Cannot add CineMatch bucket' }
& $CinematchScoopCommand install cinematch/cinematch
if ($LASTEXITCODE -ne 0) { throw 'Cannot install CineMatch through Scoop' }
& (Join-Path $CinematchScoop 'shims/cinematch.exe') --version
if ($LASTEXITCODE -ne 0) { throw 'Scoop shim cannot launch CineMatch' }
