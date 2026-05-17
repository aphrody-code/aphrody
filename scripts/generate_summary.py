import os
from pathlib import Path

def generate_summary():
    docs_dir = Path('docs')
    summary_path = docs_dir / 'SUMMARY.md'

    # Ensure a root README.md exists as the entry point
    readme_path = docs_dir / 'README.md'
    if not readme_path.exists():
        readme_path.write_text('# Google OS & CLI Documentation\n\nWelcome to the unified documentation.\n', encoding='utf-8')

    lines = ['# Summary\n', '- [Accueil](README.md)']

    # Collect all markdown files
    md_files = sorted(docs_dir.rglob('*.md'))

    # Organize by directories
    folders = {}
    for md_file in md_files:
        if md_file.name == 'SUMMARY.md' or md_file.name == 'README.md' and md_file.parent == docs_dir:
            continue

        rel_path = md_file.relative_to(docs_dir)
        folder = str(rel_path.parent)
        if folder == '.':
            folder = 'Root'

        if folder not in folders:
            folders[folder] = []
        folders[folder].append((md_file.stem, str(rel_path).replace('\\', '/')))

    for folder, files in sorted(folders.items()):
        if folder == 'Root':
            for title, path in files:
                lines.append(f'- [{title}]({path})')
        else:
            folder_title = folder.replace('\\', '/').title()
            lines.append(f'- [{folder_title}]()')
            for title, path in files:
                lines.append(f'  - [{title}]({path})')

    summary_path.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    print("Generated docs/SUMMARY.md")

if __name__ == '__main__':
    generate_summary()
