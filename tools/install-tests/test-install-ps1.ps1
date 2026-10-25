# Exercise website/docs/install.ps1 against the published releases.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\install-tests\test-install-ps1.ps1
#   pwsh -NoProfile -File tools/install-tests/test-install-ps1.ps1
#
# Needs the GitHub CLI (gh) to read the release list and to build a local
# mirror, which stands in for PYFARO_DOWNLOAD_BASE in the tamper tests.
# The installer runs in a child process of this same PowerShell, so exit codes
# are the ones a user's shell would see.

$ErrorActionPreference = 'Stop'
$Script = Join-Path $PSScriptRoot '..\..\website\docs\install.ps1'
$Engine = (Get-Process -Id $PID).Path
$Repo = 'pyfaro/pyfaro'
$Pinned = '0.3.0-alpha.1'
$Work = Join-Path ([IO.Path]::GetTempPath()) ("pyfaro-install-test-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Work | Out-Null

function Fail($Message) { throw "FAIL: $Message" }

# Run install.ps1 as a file in a child PowerShell with the given environment.
function Invoke-Installer($Environment) {
    $Saved = @{}
    foreach ($Key in $Environment.Keys) {
        $Saved[$Key] = [Environment]::GetEnvironmentVariable($Key)
        [Environment]::SetEnvironmentVariable($Key, $Environment[$Key])
    }
    try {
        & $Engine -NoProfile -ExecutionPolicy Bypass -File $Script | Out-Host
        return $LASTEXITCODE
    } finally {
        foreach ($Key in $Saved.Keys) { [Environment]::SetEnvironmentVariable($Key, $Saved[$Key]) }
    }
}

function Assert-Version($Dir, $Expected) {
    $Actual = & (Join-Path $Dir 'pyfaro.exe') --version
    if ($Actual -ne "pyfaro $Expected") { Fail "expected pyfaro $Expected in $Dir, got: $Actual" }
}

function Get-MirrorUri($Dir) { ([Uri]((Resolve-Path $Dir).Path + '\')).AbsoluteUri.TrimEnd('/') }

# Run the installer expecting a failure that installs nothing.
function Assert-Refused($Label, $Base) {
    $Dir = Join-Path $Work "refused-$Label"
    $Code = Invoke-Installer @{ PYFARO_VERSION = $Pinned; PYFARO_DOWNLOAD_BASE = $Base; PYFARO_INSTALL_DIR = $Dir }
    if ($Code -eq 0) { Fail "${Label}: the installer succeeded" }
    if (Test-Path $Dir) { Fail "${Label}: the installer left $Dir behind" }
}

try {
    Write-Host "== pinned version ($($PSVersionTable.PSVersion))"
    $Bin = Join-Path $Work 'bin'
    if ((Invoke-Installer @{ PYFARO_VERSION = $Pinned; PYFARO_INSTALL_DIR = $Bin }) -ne 0) { Fail 'pinned install failed' }
    Assert-Version $Bin $Pinned

    Write-Host '== newest release upgrades in place'
    $Newest = (gh release list --repo $Repo --limit 1 --json tagName --jq '.[0].tagName').TrimStart('v')
    if ((Invoke-Installer @{ PYFARO_VERSION = ''; PYFARO_INSTALL_DIR = $Bin }) -ne 0) { Fail 'newest install failed' }
    Assert-Version $Bin $Newest

    Write-Host '== a pinned version downgrades'
    if ((Invoke-Installer @{ PYFARO_VERSION = "v$Pinned"; PYFARO_INSTALL_DIR = $Bin }) -ne 0) { Fail 'downgrade failed' }
    Assert-Version $Bin $Pinned

    Write-Host '== piped to Invoke-Expression'
    $Piped = Join-Path $Work 'piped'
    $env:PYFARO_VERSION = $Pinned
    $env:PYFARO_INSTALL_DIR = $Piped
    try { Get-Content -Raw $Script | Invoke-Expression } finally {
        Remove-Item Env:PYFARO_VERSION, Env:PYFARO_INSTALL_DIR
    }
    Assert-Version $Piped $Pinned

    Write-Host '== local mirror'
    $Mirror = Join-Path $Work 'mirror'
    New-Item -ItemType Directory -Path (Join-Path $Mirror "v$Pinned") | Out-Null
    gh release download "v$Pinned" --repo $Repo --dir (Join-Path $Mirror "v$Pinned") --pattern 'pyfaro-*' --pattern SHA256SUMS
    $Mirrored = Join-Path $Work 'mirrored'
    if ((Invoke-Installer @{ PYFARO_VERSION = $Pinned; PYFARO_DOWNLOAD_BASE = (Get-MirrorUri $Mirror); PYFARO_INSTALL_DIR = $Mirrored }) -ne 0) { Fail 'mirror install failed' }
    Assert-Version $Mirrored $Pinned

    Write-Host '== corrupted archive is refused'
    $Corrupt = Join-Path $Work 'corrupt'
    Copy-Item -Recurse $Mirror $Corrupt
    foreach ($Archive in Get-ChildItem (Join-Path $Corrupt "v$Pinned") -Filter '*.zip') {
        [IO.File]::WriteAllBytes($Archive.FullName, [IO.File]::ReadAllBytes($Archive.FullName) + [byte[]](0x78))
    }
    Assert-Refused 'corrupt-archive' (Get-MirrorUri $Corrupt)

    Write-Host '== wrong checksum is refused'
    $WrongSum = Join-Path $Work 'wrongsum'
    Copy-Item -Recurse $Mirror $WrongSum
    $Sums = Join-Path $WrongSum "v$Pinned\SHA256SUMS"
    (Get-Content $Sums) -replace '^[0-9a-f]{64}', ('0' * 64) | Set-Content $Sums
    Assert-Refused 'wrong-checksum' (Get-MirrorUri $WrongSum)

    Write-Host '== missing checksum entry is refused'
    $NoSum = Join-Path $Work 'nosum'
    Copy-Item -Recurse $Mirror $NoSum
    Set-Content (Join-Path $NoSum "v$Pinned\SHA256SUMS") ''
    Assert-Refused 'missing-entry' (Get-MirrorUri $NoSum)

    Write-Host '== plain HTTP mirror is refused'
    Assert-Refused 'http' 'http://127.0.0.1:9'

    Write-Host "OK ($($PSVersionTable.PSVersion))"
    # The last child process failed on purpose; the runner would exit with its code.
    $global:LASTEXITCODE = 0
} finally {
    Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
