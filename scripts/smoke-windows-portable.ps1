param([Parameter(Mandatory=$true)][string]$Executable)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class ConsoleWindows {
    delegate bool Callback(IntPtr hwnd, IntPtr arg);
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback cb, IntPtr arg);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr hwnd);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd, System.Text.StringBuilder text, int count);
    public static long[] Visible() {
        var result = new List<long>();
        EnumWindows((hwnd, arg) => {
            var text = new System.Text.StringBuilder(256);
            GetClassName(hwnd, text, text.Capacity);
            if (IsWindowVisible(hwnd) && text.ToString() == "ConsoleWindowClass") result.Add(hwnd.ToInt64());
            return true;
        }, IntPtr.Zero);
        return result.ToArray();
    }
}
'@
$baselineConsoles = @([ConsoleWindows]::Visible())
$script:extractorSamples = 0
$script:consoleSamples = 0
$script:newConsoles = @{}
function ObserveExtractors($seconds) {
    $until = [DateTime]::UtcNow.AddSeconds($seconds)
    while ([DateTime]::UtcNow -lt $until) {
        $windows = @([ConsoleWindows]::Visible())
        $script:consoleSamples++
        foreach ($handle in $windows) {
            if ($handle -notin $baselineConsoles) { $script:newConsoles[$handle] = $true }
        }
        $extractors = @(Get-CimInstance Win32_Process -Filter "name='yt-dlp.exe'" | Where-Object {$_.ParentProcessId -eq $p.Id})
        if ($extractors.Count) { $script:extractorSamples++ }
        Start-Sleep -Milliseconds 50
    }
}
$start = New-Object Diagnostics.ProcessStartInfo $Executable
$start.UseShellExecute = $false
$start.WorkingDirectory = $env:SystemRoot
foreach ($key in @('FUZZY_YTDLP','FUZZY_MPV','HTTP_PROXY','HTTPS_PROXY','ALL_PROXY','NO_PROXY')) { $start.EnvironmentVariables.Remove($key) }
$start.EnvironmentVariables['PATH'] = "$env:SystemRoot\System32;$env:SystemRoot"
$start.EnvironmentVariables['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'] = '--force-renderer-accessibility'
$p = [Diagnostics.Process]::Start($start)
try {
    Start-Sleep -Seconds 6
    $p.Refresh()
    $w = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
    function Elements { $w.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition) }
    function InvokeButton($button) { ([System.Windows.Automation.InvokePattern]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke() }
    $edit = @(Elements | Where-Object {$_.Current.ControlType.ProgrammaticName -eq 'ControlType.Edit'})[0]
    ([System.Windows.Automation.ValuePattern]$edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)).SetValue('lofi hip hop')
    InvokeButton (@(Elements | Where-Object {$_.Current.Name -eq 'Search' -and $_.Current.ControlType.ProgrammaticName -eq 'ControlType.Button'})[0])
    ObserveExtractors 40
    $plays = @(Elements | Where-Object {$_.Current.Name -like 'Play *' -and $_.Current.ControlType.ProgrammaticName -eq 'ControlType.Button'})
    if (-not $plays.Count) { throw 'No live search results' }
    "Live search results: $($plays.Count)"
    InvokeButton $plays[0]
    ObserveExtractors 30
    $playing = @(Elements | Where-Object {$_.Current.ControlType.ProgrammaticName -eq 'ControlType.Text' -and $_.Current.Name -match 'Playing'})
    if (-not $playing.Count) { throw 'Playing not observed' }
    'Playing observed'
    $children = @(Get-CimInstance Win32_Process -Filter "name='mpv.exe'" | Where-Object {$_.ParentProcessId -eq $p.Id})
    if ($children.Count -ne 1) { throw 'Expected one owned mpv' }
    "Packaged mpv: $($children[0].ExecutablePath)"
    "EnumWindows samples: $script:consoleSamples; active owned extractor samples: $script:extractorSamples; new visible console windows: $($script:newConsoles.Count)"
    if (-not $script:extractorSamples) { throw 'No active extractor observed' }
    if ($script:newConsoles.Count) { throw 'New visible console windows observed' }
    Get-NetTCPConnection -OwningProcess $children[0].ProcessId -ErrorAction SilentlyContinue | Select-Object RemoteAddress,RemotePort,State
    InvokeButton (@(Elements | Where-Object {$_.Current.Name -eq 'Stop' -and $_.Current.ControlType.ProgrammaticName -eq 'ControlType.Button'})[0])
    Start-Sleep -Seconds 2
    if (@(Get-CimInstance Win32_Process -Filter "name='mpv.exe'" | Where-Object {$_.ParentProcessId -eq $p.Id}).Count) { throw 'Stop left owned mpv running' }
    'Stop cleanup passed'
} finally {
    $p.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 2
    $p.Refresh()
    "Own application exited: $($p.HasExited)"
}
