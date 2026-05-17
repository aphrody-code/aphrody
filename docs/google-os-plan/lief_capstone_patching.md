# LIEF & Capstone: Binaire Patching (msys-2.0.dll -> google-os.dll)

## Contexte
L'objectif est d'utiliser `LIEF` (Library to Instrument Executable Formats) et `Capstone` (désassembleur) pour parser la DLL du runtime MSYS2 (`msys-2.0.dll`), trouver l'implémentation de la fonction `uname`, et forcer la DLL à retourner un système "Linux". De plus, nous modifions l'entrypoint pour injecter l'escalade de privilège (NTDLL).

## LIEF C++ API Reference
LIEF permet l'édition de PE (Portable Executables) de manière programmatique en C++.

```cpp
#include <LIEF/LIEF.hpp>

using namespace LIEF::PE;

int patch_uname(const std::string& input_dll, const std::string& output_dll) {
    std::unique_ptr<Binary> binary = Parser::parse(input_dll);
    
    // Rechercher les exports pour uname
    Export* export_dir = binary->get_export();
    for (const ExportEntry& entry : export_dir->entries()) {
        if (entry.name() == "uname") {
            // Adresse virtuelle relative (RVA) de la fonction uname
            uint32_t rva = entry.address();
            
            // Avec Capstone, on peut désassembler à cette RVA pour trouver l'offset exact
            // où la chaîne "MSYS_NT" ou "CYGWIN_NT" est poussée, puis la remplacer par "Linux"
            // (La manipulation de bytecode x86_64 se fera ici).
        }
    }
    
    // Ajout d'une section pour injecter la fonction wc_grant_divine_privileges
    Section inject_section{".google"};
    inject_section.characteristics(
        static_cast<uint32_t>(SECTION_CHARACTERISTICS::IMAGE_SCN_MEM_EXECUTE |
                              SECTION_CHARACTERISTICS::IMAGE_SCN_MEM_READ |
                              SECTION_CHARACTERISTICS::IMAGE_SCN_MEM_WRITE));
    
    binary->add_section(inject_section, PE_SECTION_TYPES::UNKNOWN);
    
    // Réécriture du binaire
    binary->write(output_dll);
    return 0;
}
```

## Stratégie d'Exécution
1. L'orchestrateur Bun exécute une fonction exposée par `native/main.cpp`.
2. Le code C++ utilise LIEF pour ouvrir `msys-2.0.dll`.
3. Il effectue le patching in-memory de la fonction `uname`.
4. Le fichier est écrit sur le disque NTFS sous le nom `google-os.dll`.
