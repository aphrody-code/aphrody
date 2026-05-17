// SPDX-License-Identifier: Apache-2.0
#![cfg(target_os = "windows")]

use std::ffi::CString;

use anyhow::{Result, anyhow};
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::Debug::WriteProcessMemory,
        LibraryLoader::{GetModuleHandleA, GetProcAddress},
        Memory::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAllocEx},
        Threading::{
            CreateRemoteThread, OpenProcess, PROCESS_CREATE_THREAD, PROCESS_QUERY_INFORMATION,
            PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
        },
    },
};

/// Injecte une DLL dans un processus cible via son PID.
pub fn inject_dll(pid: u32, dll_path: &str) -> Result<()> {
    unsafe {
        // 1. Ouvrir le processus cible
        let h_process = OpenProcess(
            PROCESS_CREATE_THREAD
                | PROCESS_QUERY_INFORMATION
                | PROCESS_VM_OPERATION
                | PROCESS_VM_WRITE
                | PROCESS_VM_READ,
            false,
            pid,
        )?;

        if h_process.is_invalid() {
            return Err(anyhow!("Impossible d'ouvrir le processus {}", pid));
        }

        // 2. Trouver l'adresse de LoadLibraryA dans kernel32.dll
        let kernel32_name = CString::new("kernel32.dll")?;
        let h_kernel32 = GetModuleHandleA(windows::core::PCSTR(kernel32_name.as_ptr().cast()))?;
        if h_kernel32.is_invalid() {
            let _ = CloseHandle(h_process);
            return Err(anyhow!("Impossible d'obtenir un handle vers kernel32.dll"));
        }

        let load_lib_name = CString::new("LoadLibraryA")?;
        let load_lib_addr =
            GetProcAddress(h_kernel32, windows::core::PCSTR(load_lib_name.as_ptr().cast()));
        if load_lib_addr.is_none() {
            let _ = CloseHandle(h_process);
            return Err(anyhow!("Impossible de trouver l'adresse de LoadLibraryA"));
        }

        // 3. Allouer de la mémoire dans le processus cible pour le chemin de la DLL
        let dll_path_c = CString::new(dll_path)?;
        let dll_path_bytes = dll_path_c.as_bytes_with_nul();
        let alloc_size = dll_path_bytes.len();

        let alloc_mem =
            VirtualAllocEx(h_process, None, alloc_size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE);

        if alloc_mem.is_null() {
            let _ = CloseHandle(h_process);
            return Err(anyhow!("Échec de VirtualAllocEx"));
        }

        // 4. Écrire le chemin de la DLL dans la mémoire allouée
        let mut bytes_written = 0;
        let write_success = WriteProcessMemory(
            h_process,
            alloc_mem,
            dll_path_bytes.as_ptr().cast(),
            alloc_size,
            Some(&mut bytes_written),
        );

        if write_success.is_err() || bytes_written != alloc_size {
            let _ = CloseHandle(h_process);
            return Err(anyhow!("Échec de WriteProcessMemory"));
        }

        // 5. Créer un thread distant pour charger la DLL
        let h_thread = CreateRemoteThread(
            h_process,
            None,
            0,
            Some(std::mem::transmute::<
                Option<unsafe extern "system" fn() -> isize>,
                unsafe extern "system" fn(*mut std::ffi::c_void) -> u32,
            >(load_lib_addr)),
            Some(alloc_mem),
            0,
            None,
        )?;

        if h_thread.is_invalid() {
            let _ = CloseHandle(h_process);
            return Err(anyhow!("Échec de CreateRemoteThread"));
        }

        // Nettoyage
        let _ = CloseHandle(h_thread);
        let _ = CloseHandle(h_process);

        Ok(())
    }
}
