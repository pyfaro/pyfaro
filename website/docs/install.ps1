# Pyfaro installer for Windows (PowerShell 5.1 and PowerShell 7+).
# Published at https://pyfaro.dev/install.ps1
#
#   powershell -ExecutionPolicy ByPass -c "irm https://pyfaro.dev/install.ps1 | iex"
#
# Prefer to read it first? Download it, inspect it, then run it:
#
#   irm https://pyfaro.dev/install.ps1 -OutFile install.ps1
#   notepad install.ps1
#   powershell -ExecutionPolicy ByPass -File .\install.ps1
#
# What it does:
#   1. checks this is 64-bit Windows and picks the release archive;
#   2. downloads the archive and SHA256SUMS over HTTPS;
#   3. verifies the archive's SHA-256 and stops if it does not match;
#   4. installs pyfaro.exe and prints what it did.
# It never edits the registry or PATH and never installs uv.
# Running it again upgrades Pyfaro in place.
#
# Environment variables:
#   PYFARO_VERSION        version to install, e.g. 0.3.0-alpha.1 (default: the
#                         newest release, prereleases included)
#   PYFARO_INSTALL_DIR    where to put pyfaro.exe (default: $HOME\.local\bin)
#   PYFARO_DOWNLOAD_BASE  release download URL prefix, for testing mirrors only.
#                         Default: https://github.com/pyfaro/pyfaro/releases/download
#                         Must start with https:// or file: (a local directory).
#   GITHUB_TOKEN          optional; sent to api.github.com only, to avoid rate
#                         limits when looking up the newest release.
#
# Everything below is inside a function and the last lines call it, so a
# download that is cut short cannot run half a script.

function Install-Pyfaro {
    $ErrorActionPreference = 'Stop'
    # The progress bar makes Invoke-WebRequest very slow on Windows PowerShell.
    $ProgressPreference = 'SilentlyContinue'

    function Say($Message) { Write-Host "pyfaro-install: $Message" }

    # Windows PowerShell 5.1 does not use TLS 1.2 by default.
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    if ($env:OS -ne 'Windows_NT') {
        throw 'this script is for Windows; on Linux and macOS use install.sh (https://pyfaro.dev/getting-started/installation/)'
    }
    # PROCESSOR_ARCHITEW6432 is set for a 32-bit PowerShell on 64-bit Windows.
    $Arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    if ($Arch -ne 'AMD64') {
        throw "unsupported CPU architecture: $Arch (the only Windows build is x86_64)"
    }
    $Target = 'x86_64-pc-windows-msvc'

    $Base = if ($env:PYFARO_DOWNLOAD_BASE) { $env:PYFARO_DOWNLOAD_BASE } else { 'https://github.com/pyfaro/pyfaro/releases/download' }
    $Base = $Base.TrimEnd('/')
    if ($Base -notmatch '^(https://|file:)') {
        throw "PYFARO_DOWNLOAD_BASE must start with https:// (or file: for a local test mirror), got: $Base"
    }
    $InstallDir = if ($env:PYFARO_INSTALL_DIR) { $env:PYFARO_INSTALL_DIR } else { Join-Path $HOME '.local\bin' }

    # Fetch one release asset: over HTTPS, or from a directory for file: mirrors.
    function Get-Asset($Name, $Destination) {
        $Url = "$Base/v$Version/$Name"
        if ($Base.StartsWith('file:')) {
            Copy-Item -LiteralPath ([Uri]$Url).LocalPath -Destination $Destination
        } else {
            Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing
        }
    }

    $Version = "$env:PYFARO_VERSION".TrimStart('v')
    if (-not $Version) {
        # The newest release, prereleases included. (/releases/latest ignores
        # them and answers 404 while every release is a prerelease.)
        $Headers = @{ 'User-Agent' = 'pyfaro-install' }
        if ($env:GITHUB_TOKEN) { $Headers['Authorization'] = "Bearer $env:GITHUB_TOKEN" }
        try {
            $Releases = Invoke-RestMethod -Uri 'https://api.github.com/repos/pyfaro/pyfaro/releases?per_page=1' -Headers $Headers
            $Version = @($Releases)[0].tag_name.TrimStart('v')
        } catch {
            throw "could not look up the newest release (rate limited? set PYFARO_VERSION, for example `$env:PYFARO_VERSION = '0.3.0-alpha.1'): $($_.Exception.Message)"
        }
    }
    if ($Version -notmatch '^[0-9A-Za-z.+-]+$') { throw "invalid version: $Version" }
    Say "installing pyfaro $Version for $Target"

    $Name = "pyfaro-$Version-$Target"
    $Archive = "$Name.zip"
    $Temp = Join-Path ([IO.Path]::GetTempPath()) ("pyfaro-install-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $Temp | Out-Null
    try {
        Get-Asset $Archive (Join-Path $Temp $Archive)
        Get-Asset 'SHA256SUMS' (Join-Path $Temp 'SHA256SUMS')

        # Verify before touching the install directory: a missing entry or a
        # mismatch stops here and nothing is installed.
        $Expected = $null
        foreach ($Line in Get-Content -LiteralPath (Join-Path $Temp 'SHA256SUMS')) {
            $Parts = $Line.Trim() -split '\s+', 2
            if ($Parts.Count -eq 2 -and $Parts[1].TrimStart('*') -eq $Archive) { $Expected = $Parts[0]; break }
        }
        if (-not $Expected) { throw "SHA256SUMS has no entry for $Archive; refusing to install" }
        if ($Expected -notmatch '^[0-9a-fA-F]{64}$') { throw "SHA256SUMS has a malformed entry for $Archive; refusing to install" }
        $Actual = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $Temp $Archive)).Hash.ToLowerInvariant()
        if ($Actual -ne $Expected) {
            throw "checksum mismatch for $Archive (expected $Expected, got $Actual); the download is corrupt or has been tampered with, nothing was installed"
        }
        Say "verified the SHA-256 of $Archive"

        Expand-Archive -LiteralPath (Join-Path $Temp $Archive) -DestinationPath (Join-Path $Temp 'extract')
        $Binary = Join-Path $Temp "extract\$Name\pyfaro.exe"
        if (-not (Test-Path -LiteralPath $Binary)) { throw "$Archive does not contain $Name\pyfaro.exe" }

        $Destination = Join-Path $InstallDir 'pyfaro.exe'
        $Previous = $null
        if (Test-Path -LiteralPath $Destination) {
            try { $Previous = (& $Destination --version) } catch { }
        }
        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        # Stage beside the destination, then rename, so a failed copy never
        # leaves a half-written pyfaro.exe behind.
        $Staged = Join-Path $InstallDir '.pyfaro.exe.new'
        try {
            Copy-Item -LiteralPath $Binary -Destination $Staged -Force
            Move-Item -LiteralPath $Staged -Destination $Destination -Force
        } catch {
            Remove-Item -LiteralPath $Staged -Force -ErrorAction SilentlyContinue
            throw "could not write $Destination (is pyfaro running? set PYFARO_INSTALL_DIR to a directory you own): $($_.Exception.Message)"
        }
    } finally {
        Remove-Item -LiteralPath $Temp -Recurse -Force -ErrorAction SilentlyContinue
    }
    if ($Previous) { Say "replaced $Previous" }
    Say "installed pyfaro $Version ($Target) to $Destination"

    $OnPath = @($env:PATH -split ';') | Where-Object { $_ -and $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') }
    if (-not $OnPath) {
        Say "$InstallDir is not on your PATH. For this session, run:"
        Write-Host ''
        Write-Host "    `$env:PATH = `"$InstallDir;`$env:PATH`""
        Write-Host ''
        Say 'To keep it for your user account, run:'
        Write-Host ''
        Write-Host "    [Environment]::SetEnvironmentVariable('PATH', `"$InstallDir;`" + [Environment]::GetEnvironmentVariable('PATH', 'User'), 'User')"
        Write-Host ''
    }

    & $Destination --version
    if ($LASTEXITCODE -ne 0) { throw "$Destination --version failed (exit code $LASTEXITCODE)" }

    $Uv = Get-Command uv -ErrorAction SilentlyContinue
    if ($Uv) {
        Say "found uv: $($Uv.Source); run 'pyfaro doctor' to check it is new enough (0.9.26 or newer)"
    } else {
        Say 'uv was not found on PATH. Pyfaro needs uv 0.9.26 or newer and never installs it for you.'
        Say 'Install uv yourself, for example:'
        Write-Host ''
        Write-Host '    powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"'
        Write-Host ''
        Say 'More options: https://docs.astral.sh/uv/getting-started/installation/'
    }
}

# Run as a file, a failure exits non-zero; piped to iex, it throws instead so
# the caller's PowerShell window stays open.
try {
    Install-Pyfaro
} catch {
    if ($PSCommandPath) {
        [Console]::Error.WriteLine("pyfaro-install: error: $($_.Exception.Message)")
        exit 1
    }
    throw
}
