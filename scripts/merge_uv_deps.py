from pathlib import Path

root_toml_path = Path("Cargo.toml")
uv_toml_path = Path("vendor/uv/Cargo.toml")

with open(root_toml_path, "r", encoding="utf-8") as f:
    root_toml = f.read()

with open(uv_toml_path, "r", encoding="utf-8") as f:
    uv_toml = f.read()

# 1. Add vendor/uv/crates/* to members
if '"vendor/uv/crates/*"' not in root_toml:
    root_toml = root_toml.replace('members = [', 'members = [\n    "vendor/uv/crates/*",')

# 2. Extract [workspace.dependencies] from uv and append to root (with path adjusted)
# Since parsing TOML preserving comments/format is hard in standard library,
# we'll use a regex or basic text processing to extract the block.

in_deps = False
uv_deps = []
for line in uv_toml.splitlines():
    if line.startswith("[workspace.dependencies]"):
        in_deps = True
        continue
    elif line.startswith("[") and in_deps:
        break

    if in_deps and line.strip():
        # Adjust paths
        if 'path = "crates/' in line:
            line = line.replace('path = "crates/', 'path = "vendor/uv/crates/')
        uv_deps.append(line)

# Append uv deps to the end of Cargo.toml if not already there
if "uv-cli =" not in root_toml:
    root_toml += "\n# --- UV Dependencies ---\n"
    root_toml += "\n".join(uv_deps) + "\n"

with open(root_toml_path, "w", encoding="utf-8") as f:
    f.write(root_toml)

print("Merged UV workspace members and dependencies into root Cargo.toml")
