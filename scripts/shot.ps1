# Снимок окна Klavesin. Работает, даже когда окно перекрыто (PrintWindow, флаг 2).
# Нужен запущенный klavesin.exe с нужными флагами (--scene/--lang/--theme).
#
# Примеры:
#   powershell -NoProfile -File scripts/shot.ps1 -Out shots\main-dark-ru.png -Flags "--theme dark --lang ru"
#   klavesin.exe --scene empty &  →  shot.ps1 -Out shots\empty.png
param(
    [Parameter(Mandatory)][string]$Out,
    [string]$Exe = "klavesin",
    [string]$Flags = ""
)
Add-Type -AssemblyName UIAutomationClient, System.Drawing
Add-Type '[System.Runtime.InteropServices.DllImport("user32.dll")] public static extern bool PrintWindow(System.IntPtr hwnd, System.IntPtr hdc, uint flags);' -Name Win32 -Namespace W
$ae = [System.Windows.Automation.AutomationElement]
$p = Get-Process $Exe -ErrorAction Stop
$cond = New-Object System.Windows.Automation.PropertyCondition($ae::ProcessIdProperty, $p.Id)
$w = $ae::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
if (-not $w) { throw "у процесса $Exe нет окон" }
$r = $w.Current.BoundingRectangle
$ww = [int]$r.Width; $hh = [int]$r.Height
$bmp = New-Object System.Drawing.Bitmap $ww, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
[W.Win32]::PrintWindow($p.MainWindowHandle, $g.GetHdc(), 2) | Out-Null
$g.ReleaseHdc(); $g.Dispose()
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Output "$Out ${ww}x$hh"
