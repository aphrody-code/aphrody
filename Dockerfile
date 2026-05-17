# syntax=docker/dockerfile:1.9
# ============================================================================
#  Aphrody — Distroless static MUSL build (Linux x86_64 + aarch64)
#  Aligned with `dist` profile in Cargo.toml (LTO fat + strip + panic=abort).
#  Hermetic: --locked --offline pour reproductibilite bit-a-bit.
#  BuildKit cache mounts pour registry/target persistents entre builds.
# ============================================================================

# --- Stage 1: Build ---------------------------------------------------------
FROM --platform=$BUILDPLATFORM clux/muslrust:nightly AS builder

ARG TARGETARCH
WORKDIR /usr/src/aphrody

# Copy manifests + lockfile first for Docker layer caching.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY .cargo ./.cargo/
COPY crates ./crates/
COPY vendor ./vendor/
COPY supply-chain ./supply-chain/
COPY deny.toml ./

# Hermetic static MUSL build of the `cli` binary, dist profile.
# --locked  : fail if Cargo.lock is stale -> no version drift.
# BuildKit cache mounts persist the Cargo registry and target/ across builds.
RUN --mount=type=cache,target=/root/.cargo/registry,sharing=locked \
    --mount=type=cache,target=/root/.cargo/git,sharing=locked \
    --mount=type=cache,target=/usr/src/aphrody/target,sharing=locked \
    case "$TARGETARCH" in \
        amd64) TRIPLE=x86_64-unknown-linux-musl ;; \
        arm64) TRIPLE=aarch64-unknown-linux-musl ;; \
        *) echo "unsupported TARGETARCH=$TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add "$TRIPLE" && \
    cargo build --locked --profile dist --target "$TRIPLE" -p aphrody && \
    cp "target/$TRIPLE/dist/aphrody" /tmp/aphrody

# --- Stage 2: Distroless runtime --------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

# Copy only the static binary -- no glibc, no shell, no root user.
COPY --from=builder --chown=nonroot:nonroot /tmp/aphrody /aphrody

# OCI labels for SBOM / provenance tooling (osv-scanner, syft, cosign).
LABEL org.opencontainers.image.title="aphrody"
LABEL org.opencontainers.image.description="Aphrody -- cross-platform Rust binary."
LABEL org.opencontainers.image.licenses="Apache-2.0"
LABEL org.opencontainers.image.source="https://github.com/aphrody-code/aphrody"
LABEL org.opencontainers.image.vendor="Aphrody Authors"
LABEL org.opencontainers.image.documentation="https://github.com/aphrody-code/aphrody/blob/main/README.md"

USER nonroot:nonroot
ENTRYPOINT ["/aphrody"]
