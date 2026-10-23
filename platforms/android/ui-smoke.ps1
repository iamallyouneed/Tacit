# Build first. This installs the test APKs, temporarily selects Tacit and restores the previous IME.
# Use an isolated emulator: ./platforms/android/ui-smoke.ps1 -Serial emulator-5560
param([Parameter(Mandatory = $true)][string] $Serial)
$ErrorActionPreference = 'Stop'
$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android/Sdk' }
$exe = if ($env:OS -eq 'Windows_NT') { '.exe' } else { '' }
$bat = if ($env:OS -eq 'Windows_NT') { '.bat' } else { '' }
$tools = Get-ChildItem (Join-Path $sdk 'build-tools') | Sort-Object { [version] $_.Name } | Select-Object -Last 1
$androidJar = Join-Path $sdk 'platforms/android-34/android.jar'
$adb = Join-Path $sdk "platform-tools/adb$exe"
$device = @('-s', $Serial)
$build = Join-Path $PSScriptRoot 'build/ui-smoke'
$classes = Join-Path $build 'classes'
$dex = Join-Path $build 'dex'
New-Item -ItemType Directory -Force $classes, $dex | Out-Null
function Run($what, [scriptblock] $block) {
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed ($LASTEXITCODE)" }
}
$classpath = @($androidJar, (Join-Path $PSScriptRoot 'build/classes')) -join [IO.Path]::PathSeparator
$sources = @('KeyboardUxSmoke.java', 'KeyboardTestActivity.java') | ForEach-Object { Join-Path $PSScriptRoot "tests/$_" }
Run 'javac' { javac --release 8 -encoding UTF-8 -classpath $classpath -d $classes @sources }
$targetClasses = Join-Path $build 'target.jar'
Run 'target classpath' { jar cf $targetClasses -C (Join-Path $PSScriptRoot 'build/classes') . }
$classFiles = Get-ChildItem $classes -Recurse -Filter *.class | ForEach-Object FullName
Run 'd8' { & "$($tools.FullName)/d8$bat" --min-api 26 --lib $androidJar --classpath $targetClasses --output $dex @classFiles }
$base = Join-Path $build 'base.apk'
Run 'aapt2' {
    & "$($tools.FullName)/aapt2$exe" link -o $base -I $androidJar --manifest (Join-Path $PSScriptRoot 'tests/AndroidManifest.xml') --min-sdk-version 26 --target-sdk-version 34
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::Open($base, 'Update')
try { [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $dex 'classes.dex'), 'classes.dex') } finally { $zip.Dispose() }
$aligned = Join-Path $build 'aligned.apk'
$apk = Join-Path $build 'tests.apk'
Run 'zipalign' { & "$($tools.FullName)/zipalign$exe" -f -p 4 $base $aligned }
$keystore = if ($env:TS_KEYSTORE) { $env:TS_KEYSTORE } else { Join-Path $PSScriptRoot 'build/debug.keystore' }
$env:TACIT_TEST_SIGN_PASS = if ($env:TS_KEYSTORE) { $env:TS_KEYSTORE_PASS } else { 'android' }
Run 'sign tests' { & "$($tools.FullName)/apksigner$bat" sign --ks $keystore --ks-pass env:TACIT_TEST_SIGN_PASS --out $apk $aligned }
Run 'install keyboard' { & $adb @device install --no-incremental --no-streaming -r (Join-Path $PSScriptRoot 'build/tacit.apk') }
Run 'install UI tests' { & $adb @device install --no-incremental --no-streaming -r $apk }
$oldIme = (Run 'current IME' { & $adb @device shell settings get secure default_input_method }).Trim()
try {
    Run 'enable test IME' { & $adb @device shell ime enable io.github.iamallyouneed.tacit/.TacitIme }
    Run 'select test IME' { & $adb @device shell ime set io.github.iamallyouneed.tacit/.TacitIme }
    $result = Run 'UI smoke' { & $adb @device shell am instrument -w io.github.iamallyouneed.tacit.tests/io.github.iamallyouneed.tacit.KeyboardUxSmoke }
    $result | Write-Output
    if (($result -join "`n") -notmatch 'PASS: toolbar/readings') { throw 'Android UI smoke did not pass' }
} finally {
    if ($oldIme -and $oldIme -ne 'null') {
        Run 'restore previous IME' { & $adb @device shell ime set $oldIme }
    }
}
