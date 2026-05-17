# NTDLL & Native Windows API Bypass

## L'API NT (Windows Native API)
L'API NT (ntdll.dll) est la couche la plus basse accessible en User Mode (Ring 3) avant la transition vers le Kernel (Ring 0 / ntoskrnl.exe). Contrairement à `kernel32.dll` (Win32 API) qui encapsule les appels en appliquant des vérifications strictes (chemin de longueur max 260, vérifications de partage, interdiction de suppression directe des fichiers verrouillés), l'appel direct à `ntdll.dll` permet d'obtenir un contrôle de niveau Système.

## Fonction Critique : `NtSetInformationFile`
Dans le cadre de l'émulateur `google-os`, le runtime POSIX (qui convertit habituellement `unlink()` vers Win32 `DeleteFileW()`) devra plutôt appeler `NtSetInformationFile`. 

### Prototype
```c
NTSTATUS NtSetInformationFile(
  HANDLE                 FileHandle,
  PIO_STATUS_BLOCK       IoStatusBlock,
  PVOID                  FileInformation,
  ULONG                  Length,
  FILE_INFORMATION_CLASS FileInformationClass
);
```

### Mécanisme de suppression absolue (FileDispositionInformation)
L'API `FileDispositionInformation` (valeur enumérale `13`) indique au cache du système de fichiers de marquer le `HANDLE` pour destruction immédiate. Dès que le handle est fermé (`CloseHandle()`), le fichier disparaît même s'il est verrouillé ou surveillé par un EDR.

```cpp
typedef struct _FILE_DISPOSITION_INFORMATION {
    BOOLEAN DeleteFile;
} FILE_DISPOSITION_INFORMATION;

FILE_DISPOSITION_INFORMATION fdi;
fdi.DeleteFile = TRUE;

// Marquer pour suppression
NtSetInformationFile(hFile, &ioStatusBlock, &fdi, sizeof(FILE_DISPOSITION_INFORMATION), FileDispositionInformation);
```

## Intégration
Dans le runtime `google-os.dll`, toutes les surcouches POSIX manipulant des I/O critiques utiliseront cette méthode non-documentée de NTDLL (déjà présente dans `C:\src\aphrody\native\main.cpp` sous le nom `wc_native_delete_file`).
