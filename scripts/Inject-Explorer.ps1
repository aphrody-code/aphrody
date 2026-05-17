[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$WorkspaceRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$DllPath = "$WorkspaceRoot\src\Winclean.ExplorerHook\build\Release\WincleanExplorerHook.dll"

Write-Host "========================================"
Write-Host " INJECTEUR WINCLEAN (CIBLE: EXPLORER.EXE)"
Write-Host "========================================"

if (-not (Test-Path $DllPath)) {
    Write-Host "⚠️ La DLL n'a pas ete trouvee dans : $DllPath"
    Write-Host "Compilation en cours via CMake..."
    Set-Location "$WorkspaceRoot\src\Winclean.ExplorerHook"
    cmake --build build

    if (-not (Test-Path $DllPath)) {
        Write-Error "Echec de la compilation C++."
        exit 1
    }
}

Write-Host "Cible : $DllPath"

# Compilation a la volee d'une classe C# P/Invoke pour appeler l'API Windows (Kernel32)
$Code = @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;

public class Injector {
    [DllImport("kernel32.dll")]
    public static extern IntPtr OpenProcess(int dwDesiredAccess, bool bInheritHandle, int dwProcessId);

    [DllImport("kernel32.dll", CharSet = CharSet.Auto)]
    public static extern IntPtr GetModuleHandle(string lpModuleName);

    [DllImport("kernel32", CharSet = CharSet.Ansi, ExactSpelling = true, SetLastError = true)]
    public static extern IntPtr GetProcAddress(IntPtr hModule, string procName);

    [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
    public static extern IntPtr VirtualAllocEx(IntPtr hProcess, IntPtr lpAddress, uint dwSize, uint flAllocationType, uint flProtect);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool WriteProcessMemory(IntPtr hProcess, IntPtr lpBaseAddress, byte[] lpBuffer, uint nSize, out UIntPtr lpNumberOfBytesWritten);

    [DllImport("kernel32.dll")]
    public static extern IntPtr CreateRemoteThread(IntPtr hProcess, IntPtr lpThreadAttributes, uint dwStackSize, IntPtr lpStartAddress, IntPtr lpParameter, uint dwCreationFlags, IntPtr lpThreadId);

    // Droits necessaires
    const int PROCESS_CREATE_THREAD = 0x0002;
    const int PROCESS_QUERY_INFORMATION = 0x0400;
    const int PROCESS_VM_OPERATION = 0x0008;
    const int PROCESS_VM_WRITE = 0x0020;
    const int PROCESS_VM_READ = 0x0010;

    const uint MEM_COMMIT = 0x00001000;
    const uint MEM_RESERVE = 0x00002000;
    const uint PAGE_READWRITE = 0x04;

    public static bool Inject(int pid, string dllPath) {
        IntPtr hProcess = OpenProcess(PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_VM_READ, false, pid);
        if (hProcess == IntPtr.Zero) return false;

        IntPtr loadLibraryAddr = GetProcAddress(GetModuleHandle("kernel32.dll"), "LoadLibraryA");
        if (loadLibraryAddr == IntPtr.Zero) return false;

        IntPtr allocMemAddress = VirtualAllocEx(hProcess, IntPtr.Zero, (uint)((dllPath.Length + 1) * Marshal.SizeOf(typeof(char))), MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);
        if (allocMemAddress == IntPtr.Zero) return false;

        UIntPtr bytesWritten;
        bool written = WriteProcessMemory(hProcess, allocMemAddress, Encoding.Default.GetBytes(dllPath), (uint)((dllPath.Length + 1) * Marshal.SizeOf(typeof(char))), out bytesWritten);
        if (!written) return false;

        IntPtr hThread = CreateRemoteThread(hProcess, IntPtr.Zero, 0, loadLibraryAddr, allocMemAddress, 0, IntPtr.Zero);
        return hThread != IntPtr.Zero;
    }
}
'@

Add-Type -TypeDefinition $Code -Language CSharp

$explorer = Get-Process explorer | Select-Object -First 1
if (-not $explorer) {
    Write-Error "Processus explorer.exe introuvable."
    exit 1
}

Write-Host "Processus explorer.exe trouve (PID: $($explorer.Id)). Tentative d'injection..."

$success = [Injector]::Inject($explorer.Id, $DllPath)

if ($success) {
    Write-Host "✅ INJECTION REUSSIE ! L'implant a ete charge dans la memoire de l'explorateur."
    Write-Host "Verifiez le fichier de log : C:\winclean\var\data\explorer_hook.log"
} else {
    Write-Error "❌ Echec de l'injection. (Peut-etre un probleme de privileges administrateur)."
}
