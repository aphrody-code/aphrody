import os
import urllib.request
import zipfile
import shutil

ZIP_URL = "https://github.com/msys2/msys2.github.io/archive/refs/heads/master.zip"
DOCS_DIR = "C:/src/aphrody/msys2_docs"
TMP_ZIP = "msys2_docs.zip"
TMP_DIR = "msys2_extract"

os.makedirs(DOCS_DIR, exist_ok=True)

print("Downloading MSYS2 repository archive...")
urllib.request.urlretrieve(ZIP_URL, TMP_ZIP)

print("Extracting archive...")
with zipfile.ZipFile(TMP_ZIP, 'r') as zip_ref:
    zip_ref.extractall(TMP_DIR)

print("Copying docs to target directory...")
# The contents are inside msys2.github.io-master/docs
src_docs = os.path.join(TMP_DIR, "msys2.github.io-master", "docs")
if os.path.exists(src_docs):
    for filename in os.listdir(src_docs):
        if filename.endswith(".md"):
            shutil.copy(os.path.join(src_docs, filename), DOCS_DIR)

print("Cleaning up...")
os.remove(TMP_ZIP)
shutil.rmtree(TMP_DIR)

print(f"Successfully downloaded MSYS2 docs to {DOCS_DIR}!")
