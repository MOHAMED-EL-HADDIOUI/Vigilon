$WshShell = New-Object -ComObject WScript.Shell

# Desktop location
$DesktopPath = [System.Environment]::GetFolderPath('Desktop')
$Shortcut = $WshShell.CreateShortcut("$DesktopPath\Vigilon.lnk")
$Shortcut.TargetPath = 'wscript.exe'
$Shortcut.Arguments = '"c:\Users\asus\OneDrive\Desktop\vigilon\vigilon-desktop.vbs"'
$Shortcut.WorkingDirectory = 'c:\Users\asus\OneDrive\Desktop\vigilon'
$Shortcut.Description = 'Vigilon - Local-First Observability & Security Dashboard'
$Shortcut.IconLocation = 'c:\Users\asus\OneDrive\Desktop\vigilon\vigilon.ico,0'
$Shortcut.Save()

# Project directory shortcut
$ProjShortcut = $WshShell.CreateShortcut('c:\Users\asus\OneDrive\Desktop\vigilon\Vigilon.lnk')
$ProjShortcut.TargetPath = 'wscript.exe'
$ProjShortcut.Arguments = '"c:\Users\asus\OneDrive\Desktop\vigilon\vigilon-desktop.vbs"'
$ProjShortcut.WorkingDirectory = 'c:\Users\asus\OneDrive\Desktop\vigilon'
$ProjShortcut.Description = 'Vigilon - Local-First Observability & Security Dashboard'
$ProjShortcut.IconLocation = 'c:\Users\asus\OneDrive\Desktop\vigilon\vigilon.ico,0'
$ProjShortcut.Save()

Write-Host "Desktop shortcut created at: $DesktopPath\Vigilon.lnk"
Write-Host "Project shortcut created at: c:\Users\asus\OneDrive\Desktop\vigilon\Vigilon.lnk"
