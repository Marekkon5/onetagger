param(
    [Parameter(Mandatory)] [string] $Dist,
    [Parameter(Mandatory)] [string] $Version
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or -not $env:RUNNER_TEMP) {
    throw 'The installer smoke test requires a disposable Windows runner.'
}

$portable = Join-Path $Dist 'OneTagger-windows.exe'
$cli = Join-Path $Dist 'OneTagger-windows-cli.exe'
$setup = Join-Path $Dist 'OneTagger-windows-setup.exe'
foreach ($file in @($portable, $cli, $setup)) {
    $peVersion = (Get-Item $file).VersionInfo
    $actual = "$($peVersion.FileMajorPart).$($peVersion.FileMinorPart).$($peVersion.FileBuildPart).$($peVersion.FilePrivatePart)"
    if ($actual -ne "$Version.0") { throw "$file has file version $actual, expected $Version.0" }
}

$cliVersion = & $cli --version
if ($LASTEXITCODE -ne 0 -or $cliVersion -ne "onetagger-cli $Version") {
    throw "Unexpected CLI version output: $cliVersion"
}

$directory = Join-Path $env:RUNNER_TEMP "onetagger-installer-smoke-$PID"
$clock = [Diagnostics.Stopwatch]::StartNew()
$installer = Start-Process -FilePath $setup -ArgumentList '/S', "/D=$directory" -Wait -PassThru
$clock.Stop()
if ($installer.ExitCode -ne 0) { throw "Installer exited with $($installer.ExitCode)" }

$installed = Join-Path $directory 'onetagger.exe'
if ((Get-FileHash $installed).Hash -ne (Get-FileHash $portable).Hash) {
    throw 'Installed desktop executable differs from the portable executable.'
}
$uninstaller = Start-Process -FilePath (Join-Path $directory 'Uninstall.exe') -ArgumentList '/S' -Wait -PassThru
if ($uninstaller.ExitCode -ne 0 -or (Test-Path $installed)) { throw 'Silent uninstall failed' }

$bytes = (Get-Item $setup).Length
Write-Output "Windows $Version packaging verified. Installer: $bytes bytes. Silent install: $($clock.Elapsed.TotalSeconds) s."
