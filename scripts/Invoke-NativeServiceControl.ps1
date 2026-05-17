<#
.SYNOPSIS
    Force l'arrêt d'un service (même récalcitrant) en utilisant l'API Native Win32 (Advapi32.dll).
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$ServiceName
)

$ErrorActionPreference = 'Stop'

Write-Host "=== NATIVE SERVICE CONTROL ===" -ForegroundColor Red
Write-Host "Cible : $ServiceName" -ForegroundColor DarkGray

$code = @'
using System;
using System.Runtime.InteropServices;

public class NativeServiceControl {
    [DllImport("advapi32.dll", EntryPoint = "OpenSCManagerW", ExactSpelling = true, CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr OpenSCManager(string machineName, string databaseName, uint dwAccess);

    [DllImport("advapi32.dll", SetLastError=true, CharSet=CharSet.Auto)]
    public static extern IntPtr OpenService(IntPtr hSCManager, string lpServiceName, uint dwDesiredAccess);

    [DllImport("advapi32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool ControlService(IntPtr hService, uint dwControl, ref SERVICE_STATUS lpServiceStatus);

    [DllImport("advapi32.dll", SetLastError=true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CloseServiceHandle(IntPtr hSCObject);

    [StructLayout(LayoutKind.Sequential)]
    public struct SERVICE_STATUS {
        public uint dwServiceType;
        public uint dwCurrentState;
        public uint dwControlsAccepted;
        public uint dwWin32ExitCode;
        public uint dwServiceSpecificExitCode;
        public uint dwCheckPoint;
        public uint dwWaitHint;
    }

    const uint SC_MANAGER_CONNECT = 0x0001;
    const uint SERVICE_STOP = 0x0020;
    const uint SERVICE_CONTROL_STOP = 0x00000001;

    public static bool ForceStop(string serviceName) {
        IntPtr scm = OpenSCManager(null, null, SC_MANAGER_CONNECT);
        if (scm == IntPtr.Zero) return false;

        IntPtr service = OpenService(scm, serviceName, SERVICE_STOP);
        if (service == IntPtr.Zero) {
            CloseServiceHandle(scm);
            return false;
        }

        SERVICE_STATUS status = new SERVICE_STATUS();
        bool success = ControlService(service, SERVICE_CONTROL_STOP, ref status);

        CloseServiceHandle(service);
        CloseServiceHandle(scm);

        return success;
    }
}
'@

Add-Type -TypeDefinition $code -Language CSharp

$result = [NativeServiceControl]::ForceStop($ServiceName)

if ($result) {
    Write-Host "[OK] Commande d'arret envoyee au Kernel." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Impossible d'arreter le service (Peut-etre n'existe-t-il pas, ou PPL l'empeche)." -ForegroundColor Red
}
