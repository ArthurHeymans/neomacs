[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$InstallerAPath,

  [Parameter(Mandatory = $true)]
  [string]$InstallerBPath,

  [switch]$ConfirmEphemeralRunner
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if (
  -not $ConfirmEphemeralRunner -or
  $env:CI -cne "true" -or
  $env:GITHUB_ACTIONS -cne "true" -or
  $env:RUNNER_ENVIRONMENT -cne "github-hosted"
) {
  throw "This destructive installer contract test may run only with -ConfirmEphemeralRunner on an ephemeral GitHub Actions runner."
}

function Invoke-Installer {
  param([string]$Path)

  $process = Start-Process -FilePath $Path -ArgumentList "/S" -PassThru -Wait
  if ($process.ExitCode -ne 0) {
    throw "installer exited with code $($process.ExitCode): $Path"
  }
}

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public static class NeomacsInstallerWindows {
  private delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

  [DllImport("user32.dll")]
  private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

  [DllImport("user32.dll")]
  private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

  [DllImport("user32.dll")]
  public static extern IntPtr GetDlgItem(IntPtr dialog, int itemId);

  [DllImport("user32.dll")]
  public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);

  public static IntPtr[] ForProcess(int expectedProcessId) {
    var result = new List<IntPtr>();
    EnumWindows(delegate(IntPtr window, IntPtr parameter) {
      uint processId;
      GetWindowThreadProcessId(window, out processId);
      if (processId == expectedProcessId) {
        result.Add(window);
      }
      return true;
    }, IntPtr.Zero);
    return result.ToArray();
  }
}
"@

function Invoke-InstallerAndCancelAtWelcome {
  param([string]$Path)

  $process = Start-Process -FilePath $Path -PassThru
  try {
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
      $process.Refresh()
      if ($process.HasExited -or $process.MainWindowHandle -ne [IntPtr]::Zero) {
        break
      }
      Start-Sleep -Milliseconds 100
    }
    if ($process.HasExited -or $process.MainWindowHandle -eq [IntPtr]::Zero) {
      throw "installer did not show its welcome page"
    }
    if (-not $process.CloseMainWindow()) {
      throw "could not request cancellation from the installer welcome page"
    }

    # MUI_ABORTWARNING displays a native Yes/No dialog. IDYES is 6 and
    # BM_CLICK is 0x00F5, so this confirms cancellation without desktop input.
    $confirmed = $false
    for ($attempt = 0; $attempt -lt 100 -and -not $process.HasExited; $attempt++) {
      foreach ($window in [NeomacsInstallerWindows]::ForProcess($process.Id)) {
        $yesButton = [NeomacsInstallerWindows]::GetDlgItem($window, 6)
        if ($yesButton -ne [IntPtr]::Zero) {
          [void][NeomacsInstallerWindows]::SendMessage(
            $yesButton,
            0x00F5,
            [IntPtr]::Zero,
            [IntPtr]::Zero
          )
          $confirmed = $true
          break
        }
      }
      if (-not $confirmed) {
        Start-Sleep -Milliseconds 100
      }
    }
    if (-not $confirmed) {
      throw "installer did not show its cancellation confirmation"
    }
    if (-not $process.WaitForExit(10000)) {
      throw "installer did not exit after cancellation"
    }
  } finally {
    if (-not $process.HasExited) {
      $process.Kill()
      $process.WaitForExit()
    }
    $process.Dispose()
  }
}

$installerA = (Resolve-Path $InstallerAPath).Path
$installerB = (Resolve-Path $InstallerBPath).Path
$installDir = Join-Path $env:LOCALAPPDATA "Programs\NEO Emacs"
$shareDir = Join-Path $installDir "share\neomacs"
$uninstaller = Join-Path $installDir "uninstall.exe"
$aOnly = Join-Path $shareDir "removed-in-b.txt"
$bOnly = Join-Path $shareDir "added-in-b.txt"
$common = Join-Path $shareDir "common.txt"
$unrelated = Join-Path $installDir "created-between-versions.txt"
$installed = $false

try {
  $installed = $true
  Invoke-Installer -Path $installerA
  if (-not (Test-Path $aOnly -PathType Leaf)) {
    throw "version A did not install its version-specific owned file"
  }

  Invoke-InstallerAndCancelAtWelcome -Path $installerB
  if (-not (Test-Path $aOnly -PathType Leaf)) {
    throw "opening and cancelling version B removed version A's payload"
  }
  if (Test-Path $bOnly) {
    throw "opening and cancelling version B installed part of version B's payload"
  }
  if ((Get-Content -Path $common -Raw) -cne "version a`n") {
    throw "opening and cancelling version B changed version A's shared payload"
  }

  Set-Content -Path $unrelated -Value "not installer owned" -NoNewline

  Invoke-Installer -Path $installerB

  if (Test-Path $aOnly) {
    throw "version B left a file owned only by version A"
  }
  if (-not (Test-Path $bOnly -PathType Leaf)) {
    throw "version B did not install its version-specific owned file"
  }
  if ((Get-Content -Path $common -Raw) -cne "version b`n") {
    throw "version B did not replace the shared payload"
  }
  if (-not (Test-Path $unrelated -PathType Leaf)) {
    throw "upgrade deleted a file not owned by either installer"
  }

  $process = Start-Process -FilePath $uninstaller -ArgumentList "/S" -PassThru -Wait
  if ($process.ExitCode -ne 0) {
    throw "version B uninstaller exited with code $($process.ExitCode)"
  }
  $installed = $false

  # An NSIS uninstaller launched with /S copies itself to %TEMP% and relaunches,
  # so -Wait above returns before the deletion happens: this poll IS the wait.
  # Give it the same 10s budget the process waits in this file already use
  # (WaitForExit(10000), and the 100-attempt loops above) rather than half of
  # it -- 5s passed on windows-latest and failed on windows-11-arm.
  #
  # Report the elapsed wait on failure. Without it a repeat failure cannot be
  # told apart from a budget that is merely still too small, which is exactly
  # the ambiguity that made the first one expensive to diagnose.
  $uninstallTimeout = [TimeSpan]::FromSeconds(10)
  $waited = [Diagnostics.Stopwatch]::StartNew()
  while ((Test-Path $bOnly) -and $waited.Elapsed -lt $uninstallTimeout) {
    Start-Sleep -Milliseconds 100
  }
  $waited.Stop()
  if (Test-Path $bOnly) {
    throw ("version B uninstaller left an installer-owned file " +
      "after waiting $([int]$waited.Elapsed.TotalMilliseconds)ms " +
      "(budget $([int]$uninstallTimeout.TotalMilliseconds)ms): $bOnly")
  }
  if (-not (Test-Path $unrelated -PathType Leaf)) {
    throw "version B uninstaller deleted an unrelated file"
  }

  Remove-Item $unrelated -Force
  Remove-Item $installDir -Force
} finally {
  if ($installed -and (Test-Path $uninstaller -PathType Leaf)) {
    Start-Process -FilePath $uninstaller -ArgumentList "/S" -Wait | Out-Null
  }
  if (Test-Path $unrelated -PathType Leaf) {
    Remove-Item $unrelated -Force
  }
  if (Test-Path $installDir) {
    Remove-Item $installDir -Recurse -Force
  }
}
