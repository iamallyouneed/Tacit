# Exercise the packaged installer with the pre-0.10 updater's actual -LocalSource argument.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$work = Join-Path ([IO.Path]::GetTempPath()) ('tacit-legacy-test-' + [guid]::NewGuid().ToString('N'))
New-Item $work -ItemType Directory | Out-Null
try {
    $stage = New-Item (Join-Path $work 'stage') -ItemType Directory
    # The old alias copied the normal installer, which would take the broken local-pack path.
    Set-Content (Join-Path $stage 'install.ps1') 'param([switch]$LocalSource); if ($LocalSource) { throw "Legacy update used local packs" }' -Encoding ascii
    Set-Content (Join-Path $stage 'tacit-config.exe') 'fixture' -Encoding ascii
    foreach ($os in @('windows', 'macos', 'linux')) {
        [IO.Compression.ZipFile]::CreateFromDirectory($stage, (Join-Path $work "tacit-$os.zip"))
    }
    & python (Join-Path $PSScriptRoot 'legacy-assets.py') $work 'v0.10.5'
    if ($LASTEXITCODE) { throw 'Legacy asset packaging failed' }
    foreach ($os in @('windows', 'macos', 'linux')) {
        $zip = Join-Path $work "typescript-$os.zip"
        $expected = (Get-Content "$zip.sha256" -Raw).Trim()
        $actual = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
        if ($expected -ne "$actual  typescript-$os.zip") { throw "Bad checksum for $os" }
        $archive = [IO.Compression.ZipFile]::OpenRead($zip)
        try {
            if (@($archive.Entries | Where-Object { $_.FullName -match '[/\\]' }).Count) { throw 'Legacy updater requires a flat zip' }
            if ($archive.GetEntry('tacit-config.exe').Length -ne (Get-Item (Join-Path $stage 'tacit-config.exe')).Length) { throw 'Package payload changed' }
        } finally { $archive.Dispose() }
        if ($os -ne 'windows' -and (Get-FileHash $zip).Hash -ne (Get-FileHash (Join-Path $work "tacit-$os.zip")).Hash) { throw "Unexpected $os package change" }
    }
    $unpacked = Join-Path $work 'unpacked'
    [IO.Compression.ZipFile]::ExtractToDirectory((Join-Path $work 'typescript-windows.zip'), $unpacked)
    function Invoke-RestMethod([string]$Uri) {
        if ($Uri -ne 'https://github.com/iamallyouneed/Tacit/releases/download/v0.10.5/install.ps1') { throw "Unpinned installer URL: $Uri" }
        return 'param([string]$Version, [string]$Languages, [switch]$LocalSource); if ($Version -ne "v0.10.5" -or $Languages -ne "en,ko" -or $LocalSource) { throw "Wrong release install arguments" }; $releaseInstall.Reached = $true'
    }
    $releaseInstall = @{ Reached = $false }
    & (Join-Path $unpacked 'install.ps1') -LocalSource
    if (-not $releaseInstall.Reached) { throw 'Official release installation was not reached' }
    # -EncodedCommand must fit the 32,767-character Windows command line. Encoding the elevated
    # script inside another encoded command doubled it and made Start-Process fail with "The
    # parameter is incorrect" before the UAC prompt.
    $installer = Get-Content (Join-Path $PSScriptRoot 'install.ps1') -Raw
    $elevatedScript = [regex]::Match($installer, "(?s)\`$elevatedScript = @'\r?\n(.*?)\r?\n'@").Groups[1].Value
    $function = [regex]::Match($installer, '(?s)function Get-EncodedElevation.*?\r?\n\}').Value
    if (-not $elevatedScript -or -not $function) { throw 'Could not read the installer encoding' }
    Invoke-Expression $function
    $tokens = $null
    $parseErrors = $null
    $ast = [Management.Automation.Language.Parser]::ParseInput($elevatedScript, [ref]$tokens, [ref]$parseErrors)
    if ($parseErrors.Count -or -not $ast.ParamBlock) { throw 'Invalid elevated installer' }
    $body = $elevatedScript.Substring($ast.ParamBlock.Extent.EndOffset).TrimStart([char[]]"`r`n")
    $powershell = (Get-Command powershell.exe).Source
    $prefix = '"' + $powershell + '" -NoProfile -ExecutionPolicy Bypass -EncodedCommand '
    foreach ($Edition in @('enko', 'enkoja')) {
        foreach ($version in @('latest', 'v0.10.5', ('v1.' + ('0' * 61)))) {
            foreach ($remove in @($false, $true)) {
                $encoded = Get-EncodedElevation $version $remove
                $length = $prefix.Length + $encoded.Length + 1 # includes the terminating NUL
                if ($length -gt 32767) { throw "Elevated command exceeds Windows' limit: $length" }
                $decoded = [Text.Encoding]::Unicode.GetString([Convert]::FromBase64String($encoded))
                [ScriptBlock]::Create($decoded) | Out-Null
                if (-not $decoded.EndsWith($body, [StringComparison]::Ordinal)) { throw 'Elevated installer body changed' }
                # Execute only the parameter prelude, never the installation or UAC path.
                $prelude = $decoded.Substring(0, $decoded.Length - $body.Length)
                $bound = & ([ScriptBlock]::Create($prelude + '; @{ Version=$Version; Edition=$Edition; Uninstall=$Uninstall; LocalSource=$LocalSource; Source=$Source }'))
                if ($bound.Version -cne $version -or $bound.Edition -cne $Edition -or
                    $bound.Uninstall -ne $remove -or $bound.LocalSource -ne $false -or $bound.Source -cne '') {
                    throw 'Elevation arguments changed'
                }
            }
        }
    }
    Write-Host 'Legacy update: pinned release installer, flat assets and checksums passed.'
} finally {
    $resolved = (Resolve-Path -LiteralPath $work).Path
    if ((Split-Path $resolved -Parent) -ne ([IO.Path]::GetTempPath()).TrimEnd('\')) { throw 'Unexpected test cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
