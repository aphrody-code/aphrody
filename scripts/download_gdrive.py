# SPDX-License-Identifier: Apache-2.0

# /// script
# dependencies = [
#     "gdown>=5.0.0",
#     "tqdm",
# ]
# ///

"""Ultra-fast concurrent Google Drive recursive folder downloader.

Uses `gdown` for listing public Google Drive folders and resolving directories,
and downloads files concurrently using a thread pool with visual progress bars.
"""

import argparse
import os
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

import gdown
from tqdm import tqdm


def main():
    parser = argparse.ArgumentParser(
        description="Ultra-fast Google Drive recursive folder downloader using parallel workers."
    )
    parser.add_argument("url_or_id", help="Google Drive folder URL or folder ID.")
    parser.add_argument(
        "-o",
        "--output",
        help="Output directory path. Defaults to a subfolder named after the remote folder.",
    )
    parser.add_argument(
        "-c",
        "--concurrency",
        type=int,
        default=8,
        help="Number of concurrent file downloads. Default is 8.",
    )
    parser.add_argument(
        "--no-resume",
        action="store_true",
        help="Overwrite already downloaded files instead of resuming.",
    )

    args = parser.parse_args()
    resume = not args.no_resume
    url_or_id = args.url_or_id

    print("Retrieving folder structure and files from Google Drive...")

    try:
        if url_or_id.startswith(("http://", "https://")):
            files = gdown.download_folder(url=url_or_id, skip_download=True, quiet=True)
        else:
            files = gdown.download_folder(id=url_or_id, skip_download=True, quiet=True)
    except Exception as e:
        sys.exit(f"Error retrieving folder contents: {e}")

    if not files:
        sys.exit("No files found in folder or folder is private/invalid.")

    # Determine the folder structure and local output paths
    # gdown returns relative paths under a folder named after the drive folder.
    first_path = files[0].path
    root_folder_name = (
        first_path.split(os.sep)[0] if os.sep in first_path else "gdrive_download"
    )

    if args.output:
        root_dir = os.path.abspath(args.output)
    else:
        root_dir = os.path.abspath(root_folder_name)

    print(f"Target directory: {root_dir}")
    os.makedirs(root_dir, exist_ok=True)

    directories = set()
    files_to_download = []

    for f in files:
        if f.id is None:
            # This is a folder entry
            rel_path = f.path
            if rel_path.startswith(root_folder_name + os.sep):
                rel_path = rel_path[len(root_folder_name + os.sep) :]
            elif rel_path == root_folder_name:
                rel_path = ""
            dest_dir = os.path.join(root_dir, rel_path)
            directories.add(dest_dir)
        else:
            # This is a file entry
            rel_path = f.path
            if rel_path.startswith(root_folder_name + os.sep):
                rel_path = rel_path[len(root_folder_name + os.sep) :]
            dest_path = os.path.join(root_dir, rel_path)
            files_to_download.append((f.id, dest_path, rel_path))

    # Pre-create all subdirectories
    for d in sorted(directories):
        os.makedirs(d, exist_ok=True)

    total_files = len(files_to_download)
    print(f"Found {total_files} files inside the folder.")

    if total_files == 0:
        print("No files to download.")
        return

    progress_lock = threading.Lock()

    # Progress bars for files and overall bytes
    pbar_bytes = tqdm(
        unit="B", unit_scale=True, desc="Total Downloaded", dynamic_ncols=True
    )
    pbar_files = tqdm(
        total=total_files, unit="file", desc="Completed Files", dynamic_ncols=True
    )

    def make_progress_callback(file_id):
        last_bytes = 0

        def callback(current, total):
            nonlocal last_bytes
            diff = current - last_bytes
            last_bytes = current
            with progress_lock:
                pbar_bytes.update(diff)

        return callback

    def download_worker(file_info):
        fid, dest_path, rel_path = file_info

        # Ensure parent folder directory exists
        os.makedirs(os.path.dirname(dest_path), exist_ok=True)

        # Quick resume check: skip if file exists and has size > 0
        if resume and os.path.exists(dest_path) and os.path.getsize(dest_path) > 0:
            return True, rel_path, None

        retries = 3
        last_err = None
        for attempt in range(1, retries + 1):
            try:
                gdown.download(
                    id=fid,
                    output=dest_path,
                    quiet=True,
                    resume=resume,
                    progress=make_progress_callback(fid),
                )
                return True, rel_path, None
            except Exception as e:
                last_err = e
                if attempt < retries:
                    # Backoff before retrying
                    time.sleep(1.5**attempt)
        return False, rel_path, str(last_err)

    # Download files concurrently using ThreadPoolExecutor
    success_count = 0
    failed_files = []

    with ThreadPoolExecutor(max_workers=args.concurrency) as executor:
        futures = {
            executor.submit(download_worker, item): item for item in files_to_download
        }

        for future in as_completed(futures):
            success, rel_path, err_msg = future.result()
            with progress_lock:
                pbar_files.update(1)
            if success:
                success_count += 1
            else:
                failed_files.append((rel_path, err_msg))

    pbar_bytes.close()
    pbar_files.close()

    print("\n--- Download Summary ---")
    print(f"Successfully downloaded: {success_count} / {total_files}")
    if failed_files:
        print(f"Failed downloads: {len(failed_files)}")
        for path, err in failed_files:
            print(f"  - {path}: {err}")
        sys.exit(1)
    else:
        print("All files downloaded successfully!")


if __name__ == "__main__":
    main()
