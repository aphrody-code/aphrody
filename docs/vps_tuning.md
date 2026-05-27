<!-- SPDX-License-Identifier: Apache-2.0 -->
# Linux VPS Performance Tuning & Build Optimization Guide

This guide provides the system configurations, sysctl networking adjustments, and build instructions to optimize the `aphrody` monorepo for a high-performance VPS running **Ubuntu 26.04 (Linux kernel 7) with 40 GB RAM**.

---

## 1. Operating System & Kernel Tuning

To support high-concurrency web serving and low-latency local model/database requests, configure the Linux kernel by editing `/etc/sysctl.d/99-aphrody.conf`:

```ini
# Socket backlog limits (handles large bursts of incoming connections)
net.core.somaxconn = 32768
net.ipv4.tcp_max_syn_backlog = 16384

# Connection recycling and reuse
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15

# Device queue backlog
net.core.netdev_max_backlog = 16384

# TCP window scale & buffer sizes (optimized for 40 GB RAM)
net.ipv4.tcp_window_scaling = 1
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# Virtual Memory (prevent swap-thrashing, use hugepages for databases)
vm.swappiness = 10
vm.dirty_background_ratio = 5
vm.dirty_ratio = 10
```

Apply these settings using:
```bash
sudo sysctl --system
```

---

## 2. File Descriptor Limits

Configure the system to allow high file descriptor limits for concurrent users and processes. Edit `/etc/security/limits.d/99-aphrody.conf`:

```ini
aphrody    soft    nofile    65536
aphrody    hard    nofile    1048576
```

Verify the limits for the running service:
```bash
cat /proc/$(pgrep -u aphrody -d ',')/limits | grep "Max open files"
```

---

## 3. Transparent Hugepages (THP)

Local vector search (LanceDB/HNSW) and text embeddings (`fastembed`/ONNX Runtime) benefit significantly from Transparent Hugepages by reducing TLB misses.

Enable THP by running:
```bash
sudo echo always > /sys/kernel/mm/transparent_hugepage/enabled
sudo echo always > /sys/kernel/mm/transparent_hugepage/defrag
```

To make this persistent, append `transparent_hugepage=always` to your kernel boot parameters in `/etc/default/grub` and update grub:
```bash
# Edit GRUB_CMDLINE_LINUX_DEFAULT to include transparent_hugepage=always
sudo update-grub
```

---

## 4. Rust Native Build Optimization

Since your VPS has a high-performance environment, compiling the Rust binary natively on the host machine yields substantial performance improvements by enabling CPU-specific optimizations (AVX2/AVX-512, FMA, AES-NI, etc.).

### Building with Native CPU Target

Run the build with the `target-cpu=native` flag and the max-aggression `dist` profile:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --profile dist -p cli
```

This compiles `aphrody` with:
- **Fat LTO** (`lto = "fat"`) for aggressive cross-crate optimization.
- **Single Codegen Unit** (`codegen-units = 1`) to maximize optimization scope.
- **Stripped Symbols** (`strip = "symbols"`) to minimize binary footprint.

### Compilation Parallelism

By default, the Cargo workspace constraints limiting parallel jobs have been commented out in `.cargo/config.toml`. Cargo will automatically use all available CPU threads on your VPS.

To manually restrict or scale compiler job allocations (e.g., if compiling in the background while the main service is running), use the `CARGO_BUILD_JOBS` environment variable or `-j` flag:

```bash
# Force compilation to use 16 cores
CARGO_BUILD_JOBS=16 cargo build --release
```

---

## 5. systemd Resource Limits

The installation script (`py/aphrody/deploy/deploy-vps.sh`) automatically queries physical memory on the host and overrides systemd's static defaults:
- **`MemoryHigh`**: Dynamically set to **75% of total RAM** (approx. 30 GB on a 40 GB VPS). Memory requests exceeding this trigger proactive page reclaiming without process termination.
- **`MemoryMax`**: Dynamically set to **90% of total RAM** (approx. 36 GB on a 40 GB VPS). This represents the hard ceiling.
- **`LimitNOFILE`**: Set to `65536`.

No manual service file modifications are required; running `./deploy-vps.sh` configures these values dynamically.
