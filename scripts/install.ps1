<#
.SYNOPSIS
  Installs (or removes) tacit: the x64 and x86 text service and the settings tool.

  One line, from anywhere (downloads the latest GitHub release, checks its SHA-256):
    irm https://github.com/iamallyouneed/tacit/releases/latest/download/install.ps1 | iex
  Remove it, or pick a version:
    & ([scriptblock]::Create((irm https://github.com/iamallyouneed/tacit/releases/latest/download/install.ps1))) -Uninstall
    & ([scriptblock]::Create((irm https://github.com/iamallyouneed/tacit/releases/latest/download/install.ps1))) -Version v0.2.0

  From a checkout (builds first; needs Rust + the i686-pc-windows-msvc target):
    .\scripts\install.ps1              install or update (English + Korean language packs)
    .\scripts\install.ps1 -Japanese    also install and turn on the Japanese pack
    .\scripts\install.ps1 -Uninstall   remove it (settings in %APPDATA%\tacit are kept)

  Registering a text service writes HKLM, so this asks for administrator rights once (UAC).
  The elevated half then downloads and checks the release itself, from GitHub: the commands it
  runs are this script's own text, handed over in memory as an encoded command, never a file
  some other process of the user could swap in before the prompt is answered. A local folder is
  only ever used by -LocalSource (development), and it says so.

  Sets Tacit as the Windows default input method (keeps an existing Tacit default profile).
  Use Win+Space to switch, or Advanced keyboard settings to change the default.
#>

param(
    [switch]$Uninstall,
    [ValidateSet('', 'enko', 'enkoja')][string]$Edition = '',
    # Release tag to install (default: the latest release).
    [string]$Version = 'latest',
    # Also install and turn on the Japanese language pack (romaji -> kana -> kanji, a 10 MB
    # dictionary). The packs go to %LOCALAPPDATA%\tacit\packs; settings.json "languages"
    # says which are on, in Esc order.
    [switch]$Japanese,
    # The languages to turn on, in Esc order, e.g. ko,en,ja. Without it a new install asks
    # (tacit-config setup) and an update keeps the languages already on.
    [string]$Languages = '',
    # Internal: the elevated half of the script.
    [switch]$Elevated,
    # Internal: install the files in -Source instead of a release (checkout builds and custom
    # releases). Those files are not checked against GitHub: development use only.
    [switch]$LocalSource,
    [string]$Source
)

$GitHubRepo = 'iamallyouneed/tacit'
$ErrorActionPreference = 'Stop'
$dest = Join-Path $env:ProgramFiles 'tacit'
$dll64 = Join-Path $dest 'tacit_tip.dll'
# The picker entries (CLSID + profile GUID, see platforms/windows/src/guids.rs): with and without Tab completion.
# Registered under English (US), not Korean: see LANGID_TACIT in guids.rs for why.
$profiles = @(
    '{E231DAD4-326F-436F-B74C-D08F1D66D822}{D7B9871C-E74A-4B4F-9791-CBD1E9E074FE}',
    '{E231DAD4-326F-436F-B74C-D08F1D66D822}{BFC1C2A9-657F-443A-B4DE-ACF8EACA9AEC}'
)

# A release tag as the release pages use them (v1.2.3): safe to put in a URL.
function Test-Version([string]$Value) {
    $Value -eq 'latest' -or ($Value.Length -le 64 -and $Value -match '^v?[0-9][0-9A-Za-z.+-]*$' -and $Value -notmatch '\.\.')
}

function Set-Keyboards([bool]$Present, [string]$DefaultTip) {
    $tips = @($profiles | ForEach-Object { "0409:$_" })
    $list = Get-WinUserLanguageList
    foreach ($ko in @($list | Where-Object { $_.LanguageTag -eq 'ko' })) {
        foreach ($p in $profiles) { [void]$ko.InputMethodTips.Remove("0412:$p") }
    }
    $en = $list | Where-Object { $_.LanguageTag -eq 'en-US' }
    if ($Present -and -not $en) {
        $list.Add('en-US')
        $en = $list | Where-Object { $_.LanguageTag -eq 'en-US' }
    }
    if ($en) {
        foreach ($p in $profiles) {
            $tip = "0409:$p"
            if ($Present -and $en.InputMethodTips -notcontains $tip) { $en.InputMethodTips.Add($tip) }
            if (-not $Present) { [void]$en.InputMethodTips.Remove($tip) }
        }
    }
    Set-WinUserLanguageList $list -Force -WarningAction SilentlyContinue
    # Registration alone leaves Windows starting new apps and sessions with the system IME.
    if ($Present) {
        if ($defaultTip -notin $tips) { $defaultTip = $tips[0] }
        Set-WinDefaultInputMethodOverride -InputTip $defaultTip
    } elseif ($defaultTip -in $tips) {
        Set-WinDefaultInputMethodOverride
    }
}

# The elevated half: one block of this script's own text, run either directly (an -Elevated
# call, e.g. a -LocalSource folder install) or as an encoded command built below. Kept
# self-contained on purpose: as an encoded command it runs in a fresh PowerShell that knows
# nothing else from this file.
$elevatedScript = @'
param([string]$Version, [switch]$Uninstall, [switch]$LocalSource, [string]$Source, [ValidateSet('enko', 'enkoja')][string]$Edition = 'enko')
$ErrorActionPreference = 'Stop'
$GitHubRepo = 'iamallyouneed/tacit'
$dest = Join-Path $env:ProgramFiles 'tacit'
$dll64 = Join-Path $dest 'tacit_tip.dll'
$dll86 = Join-Path $dest 'tacit_tip_x86.dll'
$regsvr64 = Join-Path $env:SystemRoot 'System32\regsvr32.exe'
$regsvr86 = Join-Path $env:SystemRoot 'SysWOW64\regsvr32.exe'

function Invoke-Regsvr([string]$exe, [string]$dll, [switch]$Unregister) {
    $arguments = if ($Unregister) { @('/u', '/s', "`"$dll`"") } else { @('/s', "`"$dll`"") }
    $p = Start-Process $exe -ArgumentList $arguments -Wait -PassThru
    # Unregistering something already gone (a half-finished earlier run) is not an error.
    if ($p.ExitCode -ne 0 -and -not $Unregister) { throw "regsvr32 exited with $($p.ExitCode) for $dll" }
}

# A loaded DLL can be renamed but not overwritten: move it aside, delete leftovers next time.
function Move-Aside([string]$target) {
    if (Test-Path $target) {
        $backup = "$target.old-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
        Move-Item -LiteralPath $target -Destination $backup -Force
        return $backup
    }
}

# A file a running app still has mapped (Explorer keeps the text service DLL) cannot be deleted
# now. This asks Windows to delete it at the next restart, before anything loads it. Returns
# whether Windows took the request; says so when it did not.
function Remove-AtReboot([string]$path) {
    try {
        if (-not ('RebootDelete' -as [type])) {
            $source = @"
using System;
using System.Runtime.InteropServices;
public static class RebootDelete {
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern bool MoveFileEx(string existing, string target, int flags);
    // MOVEFILE_DELAY_UNTIL_REBOOT (0x4)
    public static bool AtReboot(string path) { return MoveFileEx(path, null, 0x4); }
}
"@
            Add-Type -TypeDefinition $source -ErrorAction Stop | Out-Null
        }
        if ([RebootDelete]::AtReboot($path)) { return $true }
    } catch { }
    Write-Warning "Could not schedule $path for deletion at the next restart."
    return $false
}

# Deletes the .old-* files this installer moved aside earlier (only those are ever touched); one
# still in use is left to the next restart instead of piling up forever.
function Remove-Leftovers {
    Get-ChildItem $dest -Filter '*.old-*' -ErrorAction SilentlyContinue | ForEach-Object {
        try { Remove-Item $_.FullName -Force -ErrorAction Stop }
        catch { Remove-AtReboot $_.FullName | Out-Null }
    }
}

function Unregister-Existing {
    if (Test-Path $dll64) { Invoke-Regsvr $regsvr64 $dll64 -Unregister }
    if (Test-Path $dll86) { Invoke-Regsvr $regsvr86 $dll86 -Unregister }
}

function Install-Staged([string]$Staging) {
    foreach ($name in 'tacit_tip.dll', 'tacit_tip_x86.dll', 'tacit-config.exe') {
        if (-not (Test-Path -LiteralPath (Join-Path $Staging $name) -PathType Leaf)) { throw "Package is missing $name" }
    }
    $previousDlls = @($dll64, $dll86) | Where-Object { Test-Path -LiteralPath $_ }
    $backups = [ordered]@{}
    $installed = @()
    try {
        Unregister-Existing
        foreach ($file in Get-ChildItem -LiteralPath $Staging -File) {
            $target = Join-Path $dest $file.Name
            if (Test-Path -LiteralPath $target) { $backups[$target] = Move-Aside $target }
            Move-Item -LiteralPath $file.FullName -Destination $target
            $installed += $target
        }
        & icacls.exe $dest /grant '*S-1-15-2-1:(OI)(CI)(RX)' '*S-1-15-2-2:(OI)(CI)(RX)' | Out-Null
        if ($LASTEXITCODE) { throw 'Could not grant sandboxed apps access to the installation' }
        Invoke-Regsvr $regsvr64 $dll64
        Invoke-Regsvr $regsvr86 $dll86
    } catch {
        $failure = $_
        try {
            Unregister-Existing
            foreach ($target in $installed) { Move-Aside $target | Out-Null }
            foreach ($target in $backups.Keys) {
                Move-Item -LiteralPath $backups[$target] -Destination $target
            }
            if ($previousDlls -contains $dll64) { Invoke-Regsvr $regsvr64 $dll64 }
            if ($previousDlls -contains $dll86) { Invoke-Regsvr $regsvr86 $dll86 }
        } catch {
            throw "Installation failed: $failure. Recovery also failed: $_. Keep the .old-* files in $dest for recovery."
        }
        throw "Installation failed and was rolled back: $failure"
    }
}

# The transcript goes into a fresh random folder marked high integrity: nothing running
# without the administrator token can plant a file there or swap one under the transcript.
$logDir = Join-Path $env:TEMP ("tacit-install-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item $logDir -ItemType Directory -Force | Out-Null
try { & icacls.exe $logDir /setintegritylevel '(OI)(CI)High' 2>$null } catch { }
Start-Transcript -Path (Join-Path $logDir 'install.log') -Force | Out-Null
try {
    if ($Uninstall) {
        Unregister-Existing
        if (Test-Path $dest) {
            Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue
            if (Test-Path $dest) {
                # What is left is in use by running apps. Schedule it for the next restart: the
                # deepest first, the folder itself last (a folder goes only once it is empty).
                $scheduled = $true
                $left = Get-ChildItem $dest -Recurse -Force -ErrorAction SilentlyContinue | Sort-Object { $_.FullName.Length } -Descending
                foreach ($item in $left) { if (-not (Remove-AtReboot $item.FullName)) { $scheduled = $false } }
                if (-not (Remove-AtReboot $dest)) { $scheduled = $false }
                if ($scheduled) { Write-Warning "Some files in $dest are in use; they are deleted at the next restart." }
                else { Write-Warning "Some files in $dest are in use and could not be scheduled for deletion; delete the folder after a restart." }
            }
        }
    } else {
        # Staging and backups stay inside the admin-owned install folder.
        New-Item $dest -ItemType Directory -Force | Out-Null
        $incoming = Join-Path $dest ('.incoming-' + [guid]::NewGuid().ToString('N'))
        New-Item $incoming -ItemType Directory | Out-Null
        try {
            if ($LocalSource) {
                Write-Warning "Installing from a local folder ($Source): these files are not checked against a GitHub release. Development use only."
                foreach ($name in 'tacit_tip.dll', 'tacit_tip_x86.dll', 'tacit-config.exe') {
                    Copy-Item -LiteralPath (Join-Path $Source $name) -Destination (Join-Path $incoming $name)
                }
                foreach ($name in 'tacit.ico', 'LICENSE', 'NOTICE', 'THIRD_PARTY_NOTICES.txt', 'install.ps1', 'edition') {
                    if (Test-Path -LiteralPath (Join-Path $Source $name)) {
                        Copy-Item -LiteralPath (Join-Path $Source $name) -Destination $incoming
                    }
                }
            } else {
                # The release itself is fetched and checked here, in the elevated process: what a
                # non-elevated process downloaded (or could replace) is never installed with
                # administrator rights.
                if ($Version -ne 'latest' -and ($Version -notmatch '^v?[0-9][0-9A-Za-z.+-]*$' -or $Version -match '\.\.')) {
                    throw "'$Version' is not a release version"
                }
                [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
                $base = if ($Version -eq 'latest') { "https://github.com/$GitHubRepo/releases/latest/download" } else { "https://github.com/$GitHubRepo/releases/download/$Version" }
                Write-Host "Downloading Tacit ($Version)..."
                $web = New-Object Net.WebClient
                $name = if ($Edition -eq 'enkoja') { 'tacit-windows-ja.zip' } else { 'tacit-windows.zip' }
                $zip = $web.DownloadData("$base/$name")
                $expected = ($web.DownloadString("$base/$name.sha256") -split '\s+')[0].Trim().ToLowerInvariant()
                $sha = [Security.Cryptography.SHA256]::Create()
                $actual = (($sha.ComputeHash($zip) | ForEach-Object { $_.ToString('x2') }) -join '')
                if (-not $expected -or $actual -ne $expected) { throw "Checksum mismatch (expected '$expected', got '$actual')" }
                # Straight from memory into staging: the package is never unpacked to a folder a
                # non-elevated process could reach.
                Add-Type -AssemblyName System.IO.Compression
                $stream = [IO.MemoryStream]::new($zip)
                try {
                    $archive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Read)
                    try {
                        foreach ($entry in $archive.Entries) {
                            $name = $entry.FullName
                            if (-not $name -or $name -match '[\\/:]' -or $name -eq '..' -or $name -eq '.') {
                                throw "the release package has an unsafe path: '$name'"
                            }
                            $target = Join-Path $incoming $name
                            if (Test-Path -LiteralPath $target) { throw "the release package repeats '$name'" }
                            $from = $entry.Open()
                            try {
                                $to = [IO.File]::Create($target)
                                try { $from.CopyTo($to) } finally { $to.Dispose() }
                            } finally { $from.Dispose() }
                        }
                    } finally { $archive.Dispose() }
                } finally { $stream.Dispose() }
            }
            Install-Staged $incoming
            Remove-Leftovers
        } finally {
            $resolved = [IO.Path]::GetFullPath($incoming)
            if ((Split-Path $resolved -Parent) -ne [IO.Path]::GetFullPath($dest).TrimEnd('\')) { throw 'Unexpected staging cleanup path' }
            Remove-Item -LiteralPath $resolved -Recurse -Force
        }
    }
} finally {
    Stop-Transcript | Out-Null
}
exit 0
'@

# The encoded form of that call, for the UAC step: the elevated process gets the very text
# this file is running, not a path a same-user process could swap in between.
function Get-EncodedElevation([string]$Version, [bool]$Uninstall) {
    # -EncodedCommand takes the script text itself as UTF-16LE base64. Encoding the script
    # inside another encoded command doubled it past the 32,767-character Windows command
    # line, and Start-Process failed with "The parameter is incorrect" before the UAC prompt.
    # Pass what the param block would have taken as a short prelude instead.
    $prelude = @(
        "`$Version = '$Version'"
        "`$Edition = '$Edition'"
        "`$Uninstall = `$$(if ($Uninstall) { 'true' } else { 'false' })"
        "`$LocalSource = `$false"
        "`$Source = ''"
    ) -join "`n"
    $body = $elevatedScript -replace '(?m)^param\([^\r\n]*\)\r?\n', ''
    [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($prelude + "`n" + $body))
}

if ($Elevated) {
    # The one place the body runs straight from this file (a -LocalSource folder install, or a
    # release that predates the encoded call passing -Elevated).
    $direct = @{ Version = $Version; Uninstall = $Uninstall.IsPresent; LocalSource = $LocalSource.IsPresent; Source = $Source; Edition = $(if ($Edition) { $Edition } else { 'enko' }) }
    & ([scriptblock]::Create($elevatedScript)) @direct
    exit 0
}

# Builds from a checkout into a folder laid out like the release package.
function New-SourceStage {
    $repo = Split-Path -Parent $PSScriptRoot
    Push-Location $repo
    try {
        # Not together with tacit-cli: its embedded packs would end up in these two (+2.5 MB each).
        $features = if ($Edition -eq 'enkoja') { @('--features', 'tacit-tip/japanese,tacit-config/japanese') } else { @() }
        & cargo build --release -p tacit-tip -p tacit-config @features
        if ($LASTEXITCODE) { throw 'cargo build (x64) failed' }
        $tipFeatures = if ($Edition -eq 'enkoja') { @('--features', 'japanese') } else { @() }
        & cargo build --release -p tacit-tip --target i686-pc-windows-msvc @tipFeatures
        if ($LASTEXITCODE) { throw 'cargo build (x86) failed; run: rustup target add i686-pc-windows-msvc' }
        # Out-Host: what it prints must not become part of this function's result (the stage path).
        & cargo run --release -p tacit-packgen --bin build-packs -- crates\core\data target\packs | Out-Host
        if ($LASTEXITCODE) { throw 'building the language packs failed' }
    } finally {
        Pop-Location
    }
    $stage = New-TempDirectory
    Copy-Item (Join-Path $repo 'target\release\tacit_tip.dll') $stage
    Copy-Item (Join-Path $repo 'target\i686-pc-windows-msvc\release\tacit_tip.dll') (Join-Path $stage 'tacit_tip_x86.dll')
    Copy-Item (Join-Path $repo 'target\release\tacit-config.exe') $stage
    $stagePacks = Join-Path $stage 'packs'
    New-Item $stagePacks -ItemType Directory | Out-Null
    $packIds = if ($Edition -eq 'enkoja') { @('en', 'ko', 'ja') } else { @('en', 'ko') }
    foreach ($id in $packIds) {
        Get-ChildItem (Join-Path $repo 'target\packs') -Directory -Filter "$id-*" | Copy-Item -Destination $stagePacks -Recurse
    }
    Copy-Item (Join-Path $repo 'assets\tacit.ico') $stage
    foreach ($notice in 'LICENSE', 'NOTICE', 'THIRD_PARTY_NOTICES.txt') { Copy-Item (Join-Path $repo $notice) $stage -ErrorAction SilentlyContinue }
    Set-Content (Join-Path $stage 'edition') $Edition -Encoding ascii
    Copy-Item $PSCommandPath $stage
    return $stage
}

function New-TempDirectory {
    $dir = Join-Path $env:TEMP "tacit-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    New-Item $dir -ItemType Directory | Out-Null
    return $dir
}

# Copies the language packs next to the user's settings and turns them on. Packs live in the
# user's profile (no administrator needed to add one later); AppContainer apps (Store apps,
# sandboxed browsers) load the text service too and must be able to read them.
function Install-Packs([string]$from) {
    $packRoot = Join-Path $env:LOCALAPPDATA 'tacit\packs'
    New-Item $packRoot -ItemType Directory -Force | Out-Null
    $settingsPath = Join-Path $env:APPDATA 'tacit\settings.json'
    $settings = [pscustomobject]@{}
    if (Test-Path $settingsPath) {
        try { $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json }
        catch { Write-Warning "settings.json is not valid JSON ($($_.Exception.Message)); it will be rewritten." }
    }
    $hadJapanese = [bool]$settings.PSObject.Properties['japanese'] -and $settings.japanese -eq $true
    $ids = @('en', 'ko')
    if ($Japanese -or $hadJapanese) { $ids += 'ja' }
    if ($settings.PSObject.Properties['languages']) {
        $ids = @($settings.languages)
        if ($Japanese -and $ids -notcontains 'ja') { $ids += 'ja' }
    }
    if ($Languages) { $ids = @($Languages -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
    if ($Japanese -and $ids -notcontains 'ja') { $ids += 'ja' }
    $supported = if ($Edition -eq 'enkoja') { @('en', 'ko', 'ja') } else { @('en', 'ko') }
    if (-not $Languages -and -not $Japanese) { $ids = @($ids | Where-Object { $_ -in $supported }) }
    if (-not $ids.Count) { $ids = @('en', 'ko') }
    if (@($ids | Where-Object { $_ -notin $supported }).Count) { throw 'The language selection is not supported by this edition' }
    # Korean and Japanese need the English pack's word list, even with English turned off.
    $need = @($ids) + @('en') | Select-Object -Unique
    # A pack folder is never written in place: running apps keep its files mapped and Windows
    # refuses to overwrite those. A folder that is already there is the same version (versions
    # are immutable), so it is left alone; a new one is copied aside and moved in whole, so a
    # reader never sees a half-copied pack.
    $staging = Join-Path (Split-Path $packRoot) 'packs-incoming'
    Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
    New-Item $staging -ItemType Directory -Force | Out-Null
    foreach ($id in $need) {
        Get-ChildItem (Join-Path $from 'packs') -Directory -Filter "$id-*" -ErrorAction SilentlyContinue | ForEach-Object {
            if (-not (Test-Path (Join-Path $packRoot $_.Name))) {
                Copy-Item $_.FullName (Join-Path $staging $_.Name) -Recurse
                Move-Item (Join-Path $staging $_.Name) (Join-Path $packRoot $_.Name)
            }
        }
    }
    Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
    # On the folder above, so the staging folder and every pack folder inherit it (a folder moved
    # into place keeps the permissions it had).
    & icacls.exe (Split-Path $packRoot) /grant '*S-1-15-2-1:(OI)(CI)(RX)' '*S-1-15-2-2:(OI)(CI)(RX)' /T | Out-Null
    $settings | Add-Member -NotePropertyName languages -NotePropertyValue $ids -Force
    # The versions before language packs read this key; keep it true to what is on.
    $settings | Add-Member -NotePropertyName japanese -NotePropertyValue ($ids -contains 'ja') -Force
    New-Item (Split-Path $settingsPath) -ItemType Directory -Force | Out-Null
    # Not Set-Content -Encoding utf8: Windows PowerShell 5.1 writes a byte order mark with it.
    [IO.File]::WriteAllText($settingsPath, ($settings | ConvertTo-Json), (New-Object Text.UTF8Encoding($false)))
}

# "tacit <settings>" in Korean, built from code points: Windows PowerShell 5.1 reads a script
# file without a byte order mark as ANSI, so this file stays ASCII.
$settingsLabel = 'Tacit ' + [string][char]0xC124 + [char]0xC815

# The Start menu entry that opens the settings page (tacit-config is not on PATH).
function Set-SettingsShortcut([bool]$Present) {
    $link = Join-Path $env:APPDATA ('Microsoft\Windows\Start Menu\Programs\' + $settingsLabel + '.lnk')
    if (-not $Present) { Remove-Item $link -Force -ErrorAction SilentlyContinue; return }
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($link)
    $shortcut.TargetPath = Join-Path $dest 'tacit-config.exe'
    $shortcut.IconLocation = Join-Path $dest 'tacit.ico'
    $shortcut.Description = 'Tacit: languages, settings, updates'
    $shortcut.Save()
}

# The Han/Eng key's shortcut works everywhere (desktop, taskbar, any app) through a keyboard
# hook in its own process, started at sign-in: `rundll32 tacit_tip.dll,KeyHook`.
function Stop-KeyHook {
    Get-CimInstance Win32_Process -Filter "Name='rundll32.exe'" -ErrorAction SilentlyContinue |
        Where-Object { $_.CommandLine -like '*tacit_tip*KeyHook*' } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
}

function Set-KeyHook([bool]$Present) {
    $run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
    Stop-KeyHook
    if (-not $Present) { Remove-ItemProperty $run -Name 'tacit keys' -ErrorAction SilentlyContinue; return }
    $command = "rundll32.exe `"$dll64`",KeyHook"
    Set-ItemProperty $run -Name 'tacit keys' -Value $command
    Start-Process rundll32.exe -ArgumentList "`"$dll64`",KeyHook"
}

# The languages already on in settings.json, as a list (empty on a first install).
function Get-ConfiguredLanguages {
    $path = Join-Path $env:APPDATA 'tacit\settings.json'
    if (-not (Test-Path $path)) { return @() }
    try { $s = Get-Content $path -Raw | ConvertFrom-Json } catch { return @() }
    if ($s.PSObject.Properties['languages']) { return @($s.languages) }
    if ($s.PSObject.Properties['japanese'] -and $s.japanese -eq $true) { return @('en', 'ko', 'ja') }
    return @('en', 'ko')
}

# Set when this is a first install with no languages given: the first-run page (opened at the
# end, see Open-Welcome) lets the user choose them.
$script:FreshInstall = $false

# A release install: the packs come from the release through tacit-config. Languages that
# are given (-Languages, -Japanese) or already on are installed quietly; on a first install the
# first-run page asks instead.
function Install-ReleasePacks {
    $config = Join-Path $dest 'tacit-config.exe'
    if (-not (Test-Path $config)) {
        # A release from before tacit-config (-Version): it has no language packs to fetch.
        Write-Warning 'This release has no tacit-config, so no language packs were installed. Install a newer release to choose languages.'
        return
    }
    $localRoot = Join-Path $env:LOCALAPPDATA 'tacit'
    New-Item (Join-Path $localRoot 'packs') -ItemType Directory -Force | Out-Null
    # Store apps and sandboxed processes read the packs; set before any pack exists so they inherit it.
    & icacls.exe $localRoot /grant '*S-1-15-2-1:(OI)(CI)(RX)' '*S-1-15-2-2:(OI)(CI)(RX)' /T | Out-Null
    $previousRelease = $env:TACIT_RELEASE
    if ($Version -ne 'latest') { $env:TACIT_RELEASE = "https://github.com/$GitHubRepo/releases/download/$Version/" }
    $wanted = @($Languages -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    if (-not $wanted.Count) {
        $configured = @(Get-ConfiguredLanguages)
        $wanted = @($configured | Where-Object { $Edition -eq 'enkoja' -or $_ -ne 'ja' })
        if ($configured.Count -and -not $wanted.Count) { $wanted = @('en', 'ko') }
    }
    if ($Japanese -and $wanted.Count -and $wanted -notcontains 'ja') { $wanted += 'ja' }
    try {
        if ($wanted.Count) { & $config setup --languages ($wanted -join ',') }
        elseif ($Japanese) { & $config setup --languages 'en,ko,ja' }
        else { $script:FreshInstall = $true }
    } finally {
        # Do not leave the pin in the user's shell (this runs inside `irm | iex`).
        if ($null -eq $previousRelease) { Remove-Item Env:\TACIT_RELEASE -ErrorAction SilentlyContinue } else { $env:TACIT_RELEASE = $previousRelease }
    }
    if ($LASTEXITCODE) { throw "tacit-config setup failed (exit $LASTEXITCODE). Run it again from a terminal: tacit-config setup --languages ko,en" }
}

# The first-run page: the greeting, the language choice, a short practice. It runs on its own
# (as this user, not elevated) and ends by itself a few minutes after its tab is closed.
function Open-Welcome {
    $config = Join-Path $dest 'tacit-config.exe'
    $previousRelease = $env:TACIT_RELEASE
    # The page fetches the packs of the release being installed, not of the latest one.
    if ($Version -ne 'latest') { $env:TACIT_RELEASE = "https://github.com/$GitHubRepo/releases/download/$Version/" }
    try {
        Start-Process $config -ArgumentList 'welcome' -WindowStyle Hidden
    } finally {
        if ($null -eq $previousRelease) { Remove-Item Env:\TACIT_RELEASE -ErrorAction SilentlyContinue } else { $env:TACIT_RELEASE = $previousRelease }
    }
}

# Explicit language selection determines the edition; otherwise preserve the installed one.
if ($Uninstall) { $Edition = 'enko' }
$wanted = @($Languages -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ })
if (-not $Edition) {
    $editionFile = Join-Path $dest 'edition'
    if ($Japanese -or $wanted -contains 'ja') { $Edition = 'enkoja' }
    elseif ($wanted.Count) { $Edition = 'enko' }
    elseif (Test-Path $editionFile) { $Edition = (Get-Content $editionFile -Raw).Trim() }
    elseif ((Get-ConfiguredLanguages) -contains 'ja') { $Edition = 'enkoja' }
    else { $Edition = 'enko' }
}
if ($Edition -notin @('enko', 'enkoja')) { throw 'Invalid installed edition' }
if ($Edition -eq 'enko' -and ($Japanese -or $wanted -contains 'ja')) { throw 'The selected languages require the enkoja edition' }
$supported = if ($Edition -eq 'enkoja') { @('en', 'ko', 'ja') } else { @('en', 'ko') }
if (@($wanted | Where-Object { $_ -notin $supported }).Count) { throw 'Unsupported language selection' }
$stage = $null
$inCheckout = $PSScriptRoot -and (Test-Path (Join-Path (Split-Path -Parent $PSScriptRoot) 'Cargo.toml'))
try {
    if (-not (Test-Version $Version)) { throw "'$Version' is not a release version" }
    # Unregistering the old DLL can clear its default; remember the user's profile first.
    $defaultInputTip = (Get-WinDefaultInputMethodOverride).InputMethodTip
    if ($LocalSource -or $inCheckout) {
        if ($inCheckout) {
            $stage = New-SourceStage
            $Source = $stage
        } elseif (-not $Source) {
            $Source = $PSScriptRoot
        }
        $script = if ($PSCommandPath) { $PSCommandPath } else { Join-Path $Source 'install.ps1' }
        Write-Warning 'Installing from a local folder: these files are not checked against a GitHub release.'
        $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$script`"", '-Elevated', '-LocalSource', '-Source', "`"$Source`"", '-Edition', $Edition)
        if ($Uninstall) { $arguments += '-Uninstall' }
    } else {
        # A release install (or removal): the elevated half fetches and checks the release
        # itself; nothing from this side, or from any user-writable folder, goes over.
        $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', (Get-EncodedElevation $Version $Uninstall.IsPresent))
    }
    # An uninstall removes the folder its process holds the DLL in; an install only renames the
    # loaded DLL, and Set-KeyHook below restarts the process on the new one.
    if ($Uninstall) { Stop-KeyHook }

    Write-Host 'Asking for administrator rights to register the text service...'
    $p = Start-Process powershell.exe -Verb RunAs -ArgumentList $arguments -Wait -PassThru
    if ($p.ExitCode -ne 0) {
        # Re-registering the restored DLL can clear the previous per-user default.
        if ($defaultInputTip) { Set-WinDefaultInputMethodOverride -InputTip $defaultInputTip }
        throw "The elevated step failed (exit $($p.ExitCode)). A transcript, when the step ran, is in $env:TEMP\tacit-install-*."
    }

    if ($Uninstall) {
        Set-Keyboards $false $defaultInputTip
        Set-SettingsShortcut $false
        Set-KeyHook $false
        Write-Host 'Tacit removed. Settings stay in %APPDATA%\tacit, language packs in %LOCALAPPDATA%\tacit.'
    } else {
        $packSource = if ($stage) { $stage } else { $Source }
        if ($LocalSource -or $inCheckout) { Install-Packs $packSource } else { Install-ReleasePacks }
        # Per user: switch the registered profile on.
        & rundll32.exe $dll64, EnableForUser
        Set-Keyboards $true $defaultInputTip
        Set-SettingsShortcut $true
        Set-KeyHook $true
        Write-Host 'Tacit installed.'
        Write-Host '  Win+Space  -> "Tacit (autocomplete)" or "Tacit" (no autocomplete)'
        Write-Host '  Switch word language -> Esc or Han/Eng; Pause -> Alt+Han/Eng, per text field. Keys: dashboard settings, or %APPDATA%\tacit\settings.json'
        Write-Host ('  Languages and settings: Start menu > ' + $settingsLabel + ' (opens a page in your browser), or ' + (Join-Path $dest 'tacit-config.exe'))
        Write-Host '  Apps already running pick the new text service up when you reopen them.'
        if ($script:FreshInstall) {
            Write-Host 'Opening the first-run page in your browser (choose languages, try it out)...'
            Open-Welcome
        }
    }
} finally {
    if ($stage) { Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue }
}

