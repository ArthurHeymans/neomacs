param(
  [switch]$Install
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($env:GSTREAMER_VERSION)) {
  throw "GSTREAMER_VERSION is not set"
}

$installRoot = "C:\gstreamer"
$msiCacheRoot = "C:\gstreamer-msi-cache"
$baseUrl = "https://gstreamer.freedesktop.org/data/pkg/windows/$env:GSTREAMER_VERSION/msvc"
$runtimeMsi = Join-Path $msiCacheRoot "gstreamer-1.0-msvc-x86_64-$env:GSTREAMER_VERSION.msi"
$develMsi = Join-Path $msiCacheRoot "gstreamer-1.0-devel-msvc-x86_64-$env:GSTREAMER_VERSION.msi"

function Download-IfMissing($uri, $path) {
  if (Test-Path $path) {
    return
  }
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
  Invoke-WebRequest -Uri $uri -OutFile $path
}

function Install-Msi($path) {
  $process = Start-Process msiexec.exe -Wait -PassThru -ArgumentList @(
    "/i", $path, "/qn", "INSTALLLEVEL=1000", "INSTALLDIR=$installRoot"
  )
  if ($process.ExitCode -ne 0) {
    throw "msiexec failed with exit code $($process.ExitCode): $path"
  }
}

function Export-CiEnv($name, $value) {
  if ($env:GITHUB_ENV) {
    Add-Content -Path $env:GITHUB_ENV -Value "$name=$value"
  } else {
    Set-Item -Path "Env:$name" -Value $value
  }
}

function Export-CiPath($value) {
  if ($env:GITHUB_PATH) {
    Add-Content -Path $env:GITHUB_PATH -Value $value
  } else {
    $env:PATH = "$value;$env:PATH"
  }
}

if ($Install) {
  Download-IfMissing "$baseUrl/gstreamer-1.0-msvc-x86_64-$env:GSTREAMER_VERSION.msi" $runtimeMsi
  Download-IfMissing "$baseUrl/gstreamer-1.0-devel-msvc-x86_64-$env:GSTREAMER_VERSION.msi" $develMsi
  Install-Msi $runtimeMsi
  Install-Msi $develMsi
}

$searchRoots = @($installRoot, "${env:ProgramFiles}\gstreamer", "${env:ProgramFiles(x86)}\gstreamer") |
  Where-Object { Test-Path $_ }
$glibPc = $searchRoots |
  ForEach-Object { Get-ChildItem -Path $_ -Filter glib-2.0.pc -Recurse -ErrorAction SilentlyContinue } |
  Select-Object -First 1

if (-not $glibPc) {
  $searchRoots | ForEach-Object { Get-ChildItem -Path $_ -Depth 4 -ErrorAction SilentlyContinue }
  throw "glib-2.0.pc not found; restore or install the GStreamer devel MSI first"
}

$pkgConfigDir = Split-Path -Parent $glibPc.FullName
$libDir = Split-Path -Parent $pkgConfigDir
$gstRoot = Split-Path -Parent $libDir
$pkgConfig = "$gstRoot\bin\pkg-config.exe"

if (-not (Test-Path $pkgConfig)) {
  choco install pkgconfiglite -y
  $pkgConfig = (Get-Command pkg-config.exe -All |
    Where-Object { $_.Source -notmatch "\\Git\\" } |
    Select-Object -First 1 -ExpandProperty Source)
  if (-not $pkgConfig) {
    throw "pkg-config.exe not found after installing pkgconfiglite"
  }
}

if (-not (Test-Path $runtimeMsi)) {
  Download-IfMissing "$baseUrl/gstreamer-1.0-msvc-x86_64-$env:GSTREAMER_VERSION.msi" $runtimeMsi
}

Export-CiPath (Split-Path -Parent $pkgConfig)
Export-CiPath "$gstRoot\bin"
Export-CiEnv "GSTREAMER_ROOT_X86_64" "$gstRoot\"
Export-CiEnv "PKG_CONFIG" $pkgConfig
Export-CiEnv "PKG_CONFIG_PATH" "$gstRoot\lib\pkgconfig"
Export-CiEnv "GSTREAMER_RUNTIME_MSI" $runtimeMsi
