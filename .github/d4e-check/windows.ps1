# Temporary (D4e manual check on a hosted runner; deleted before merge): real mouse drags of a tab
# through SendInput, out of its window and onto another.
$ErrorActionPreference = 'Continue'
$out = 'target/d4e-check'; New-Item -ItemType Directory -Force $out | Out-Null
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text; using System.Collections.Generic;
public static class W {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr p);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool r);
  public static List<string> List() {
    var l = new List<string>();
    EnumWindows((h, p) => {
      var s = new StringBuilder(256); GetWindowText(h, s, 256);
      if (IsWindowVisible(h) && s.ToString().Contains("Lamina PDF")) {
        RECT r; GetClientRect(h, out r); var o = new POINT(); ClientToScreen(h, ref o);
        l.Add(h + "|" + s + "|" + o.X + "|" + o.Y + "|" + r.R + "|" + r.B);
      }
      return true; }, IntPtr.Zero);
    return l;
  }
}
"@
function Shot($name) {
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
  $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
  $bmp.Save("$out/$name.png"); $g.Dispose(); $bmp.Dispose()
}
function Windows() { [W]::List() | ForEach-Object { Write-Output $_ } }
function Drag($x1, $y1, $x2, $y2, $name) {
  [W]::SetCursorPos($x1, $y1); Start-Sleep -Milliseconds 300
  [W]::mouse_event(0x2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 300
  for ($i = 1; $i -le 30; $i++) { [W]::SetCursorPos([int]($x1 + ($x2 - $x1) * $i / 30), [int]($y1 + ($y2 - $y1) * $i / 30)); Start-Sleep -Milliseconds 40 }
  Start-Sleep -Milliseconds 1000; Shot $name
  [W]::mouse_event(0x4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Seconds 3
}
$ex = (Resolve-Path fixtures/golden/examples).Path
Start-Process target/debug/laminapdf-desktop.exe -ArgumentList "`"$ex\edit_bookmarks.pdf`"", "`"$ex\edit_metadata.pdf`"" -RedirectStandardOutput "$out/app.log" -RedirectStandardError "$out/app.err"
Start-Sleep -Seconds 25
Write-Output '## start'; Windows; Shot '1-start'
$m = ([W]::List()[0]).Split('|')
[W]::MoveWindow([IntPtr][long]$m[0], 0, 0, 660, 500, $true) | Out-Null; Start-Sleep -Seconds 2
$m = ([W]::List() | Where-Object { $_.Split('|')[0] -eq $m[0] }).Split('|'); $mx = [int]$m[2]; $my = [int]$m[3]
Write-Output "main client at $mx,$my"
Drag ($mx + 301) ($my + 16) 760 250 '2-mid-out'
Write-Output '## after drag out'; Windows; Shot '3-after-out'
$all = [W]::List()
$main = ($all | Where-Object { $_.Split('|')[0] -eq $m[0] }).Split('|')
$new = ($all | Where-Object { $_.Split('|')[0] -ne $m[0] } | Select-Object -First 1)
if ($new) {
  $n = $new.Split('|')
  [W]::SetForegroundWindow([IntPtr][long]$main[0]) | Out-Null; Start-Sleep -Seconds 1
  Drag ([int]$main[2] + 108) ([int]$main[3] + 16) ([int]$n[2] + 100) ([int]$n[3] + 150) '4-mid-onto'
  Write-Output '## after drag onto'; Windows; Shot '5-after-onto'
}
Get-Content "$out/app.log", "$out/app.err" -ErrorAction SilentlyContinue | Select-String -NotMatch 'tile' | Select-Object -Last 20
