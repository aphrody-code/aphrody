import os
import shutil
import re
from pathlib import Path

crates_dir = Path("crates")
n2b_crates = [
    "n2b-ai", "n2b-cli", "n2b-core", "n2b-github",
    "n2b-registry", "n2b-report", "n2b-rules", "n2b-scanners",
    "n2b-types", "n2b-util"
]

n2b_new_dir = crates_dir / "n2b"
n2b_new_src = n2b_new_dir / "src"

# 1. Gather all dependencies
all_deps = set()
features = set()

def parse_toml_deps(toml_path):
    if not toml_path.exists(): return
    with open(toml_path, "r", encoding="utf-8") as f:
        content = f.read()

    in_deps = False
    for line in content.split("\n"):
        line = line.strip()
        if line.startswith("[dependencies]"):
            in_deps = True
            continue
        elif line.startswith("[") and in_deps:
            in_deps = False

        if in_deps and line and not line.startswith("#"):
            # skip self-dependencies
            if "n2b-" in line:
                continue
            all_deps.add(line)

for c in n2b_crates:
    parse_toml_deps(crates_dir / c / "Cargo.toml")

# 2. Create new crate structure
os.makedirs(n2b_new_src, exist_ok=True)

with open(n2b_new_dir / "Cargo.toml", "w", encoding="utf-8") as f:
    f.write("""[package]
name = "n2b"
version = "0.5.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lib]
name = "n2b"
path = "src/lib.rs"

[dependencies]
""")
    for dep in sorted(all_deps):
        f.write(dep + "\n")

    f.write("""
[dev-dependencies]
proptest = "1"
serde_json = { workspace = true }

[features]
default = []
""")

# 3. Move source files and create modules
lib_rs_content = []

for c in n2b_crates:
    mod_name = c.replace("n2b-", "")
    lib_rs_content.append(f"pub mod {mod_name};")

    old_src = crates_dir / c / "src"
    new_mod_dir = n2b_new_src / mod_name
    os.makedirs(new_mod_dir, exist_ok=True)

    if old_src.exists():
        for item in os.listdir(old_src):
            s = old_src / item
            d = new_mod_dir / item
            if s.is_file():
                if item == "main.rs":
                    continue # skip main if it exists
                if item == "lib.rs":
                    d = new_mod_dir / "mod.rs"
                shutil.copy2(s, d)
            elif s.is_dir():
                shutil.copytree(s, d)

with open(n2b_new_src / "lib.rs", "w", encoding="utf-8") as f:
    f.write("\n".join(lib_rs_content) + "\n")

# 4. Refactor imports inside n2b
def replace_imports_in_file(path):
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    # replace n2b_foo:: with crate::foo::
    for c in n2b_crates:
        mod_name = c.replace("n2b-", "")
        old_import = c.replace("-", "_") + "::"
        new_import = f"crate::{mod_name}::"
        content = content.replace(old_import, new_import)

        # also replace `use n2b_foo;`
        content = re.sub(rf'\bn2b_{mod_name}\b', f'crate::{mod_name}', content)

    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

for root, _, files in os.walk(n2b_new_src):
    for file in files:
        if file.endswith(".rs"):
            replace_imports_in_file(os.path.join(root, file))

# 5. Delete old crates
for c in n2b_crates:
    shutil.rmtree(crates_dir / c, ignore_errors=True)

print("Consolidation complete.")
