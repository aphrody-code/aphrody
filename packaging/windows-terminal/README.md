<!-- SPDX-License-Identifier: Apache-2.0 -->
# Windows Terminal profile for aphrody

## Install via JSON fragment (recommended)

Windows Terminal loads fragment files automatically from:

```
%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\<app-name>\<file>.json
```

Drop `aphrody.profile.json` as:

```
%LOCALAPPDATA%\Microsoft\Windows Terminal\Fragments\aphrody-code\aphrody.json
```

The profile named **aphrody** will appear in the new-tab dropdown on the next
Terminal launch — no restart of Windows Terminal required on recent builds.

PowerShell one-liner:

```powershell
$dest = "$env:LOCALAPPDATA\Microsoft\Windows Terminal\Fragments\aphrody-code"
New-Item -ItemType Directory -Path $dest -Force | Out-Null
Copy-Item "$PSScriptRoot\aphrody.profile.json" "$dest\aphrody.json" -Force
```

## Install manually (settings.json)

1. Open Windows Terminal settings (`Ctrl+,`).
2. Click **Open JSON file** (bottom-left).
3. Locate the `"profiles"` → `"list"` array and paste the object from the
   `"profiles"` array in `aphrody.profile.json` as a new element.
4. Save and reload.

## Future: auto-drop via install.ps1

`packaging/install.ps1` could copy this fragment to the Fragments directory as
a post-install step, giving users the profile automatically after `irm … | iex`.
