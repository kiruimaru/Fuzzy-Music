param(
    [ValidateSet('search', 'resolve')]
    [string]$Mode = 'search',
    [string]$Query = 'lofi hip hop',
    [ValidateRange(1, 20)]
    [int]$Limit = 5,
    [ValidateRange(0.001, 3600)]
    [double]$TimeoutSeconds = 45,
    [ValidateRange(0, 3600)]
    [double]$CancelAfterSeconds = 0
)
$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$cargoCommand = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCommand) {
    $cargoExecutable = $cargoCommand.Source
} else {
    $cargoExecutable = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
    if (-not (Test-Path -LiteralPath $cargoExecutable)) { throw 'Install Rust using rustup before running this check.' }
}
& (Join-Path $PSScriptRoot 'setup-ytdlp.ps1')
$extractor = Join-Path $repositoryRoot 'tools/yt-dlp.exe'
$arguments = @('run', '--quiet', '--locked', '--manifest-path', (Join-Path $repositoryRoot 'Cargo.toml'), '-p', 'fuzzy-soundcloud', '--bin', 'soundcloud-probe', '--', $Mode, $Query, '--yt-dlp', $extractor)
if ($Mode -eq 'search') { $arguments += @('--limit', [string]$Limit) }
$arguments += @('--timeout', $TimeoutSeconds.ToString([Globalization.CultureInfo]::InvariantCulture))
if ($CancelAfterSeconds -gt 0) { $arguments += @('--cancel-after', $CancelAfterSeconds.ToString([Globalization.CultureInfo]::InvariantCulture)) }
& $cargoExecutable @arguments
exit $LASTEXITCODE
