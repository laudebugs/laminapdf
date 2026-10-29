# The Linux (ARM64) CI runner image for lamina (todo X5, tools/ci-runner/linux/README.md).
# GitHub's runner image, pinned by digest (Dependabot's docker ecosystem bumps it), plus the
# system packages `cargo xtask check` needs and the Rust toolchain pinned in rust-toolchain.toml,
# so jobs don't download it. Node, pnpm, cargo-nextest and cargo-deny come from the workflow.
# Every job runs in a new container from this image and nothing it writes survives the job: no
# volumes. loop.sh builds the image from main's copy of this directory and of
# rust-toolchain.toml (put next to this file in the build context), and rebuilds it when either
# changes. No credentials go into the image.
FROM ghcr.io/actions/actions-runner:2.338.2@sha256:759a1d94a15b05941aba5196974ff43fe15b4cbba29b694b35cbc32a1166b232

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        curl \
        git \
        jq \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY --chmod=0755 entrypoint.sh /usr/local/bin/lamina-runner-entrypoint

USER runner
WORKDIR /home/runner
ENV CARGO_HOME=/home/runner/.cargo \
    RUSTUP_HOME=/home/runner/.rustup
ENV PATH=/home/runner/.cargo/bin:$PATH

# rustup-init, pinned by version and SHA-256 (from static.rust-lang.org/rustup/archive/).
ARG RUSTUP_VERSION=1.28.2
ARG RUSTUP_INIT_SHA256=e3853c5a252fca15252d07cb23a1bdd9377a8c6f3efa01531109281ae47f841c
COPY --chown=runner:runner rust-toolchain.toml /home/runner/rust-pin/rust-toolchain.toml
# `rustup toolchain install` with no argument installs what rust-toolchain.toml names: the
# version, its components and its targets.
RUN set -eux; \
    curl --proto '=https' --tlsv1.2 -fsSLo /tmp/rustup-init \
        "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/aarch64-unknown-linux-gnu/rustup-init"; \
    echo "${RUSTUP_INIT_SHA256}  /tmp/rustup-init" | sha256sum -c -; \
    chmod +x /tmp/rustup-init; \
    /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none; \
    rm /tmp/rustup-init; \
    cd /home/runner/rust-pin; \
    rustup toolchain install; \
    rustup default "$(sed -n 's/^channel = "\(.*\)".*/\1/p' rust-toolchain.toml)"; \
    rustc --version

ENTRYPOINT ["/usr/local/bin/lamina-runner-entrypoint"]
