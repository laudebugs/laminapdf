# The Linux (ARM64) CI runner image for lamina (todo X5, tools/ci-runner/linux/README.md).
# GitHub's runner image, pinned by digest (Dependabot's docker ecosystem bumps it), plus the
# system packages `cargo xtask check` needs. Rust, Node and pnpm are not baked in: the workflow
# installs them, as on the macOS runners; Rust lands in the two named volumes below, so it is
# only downloaded once. No credentials go into the image: the container gets a single-use JIT
# runner config at start (see loop.sh).
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

# Fixed homes for rustup and cargo, mounted as the named volumes lamina-linux-rustup and
# lamina-linux-cargo. Creating them here, owned by `runner`, gives a fresh volume that owner.
ENV CARGO_HOME=/home/runner/.cargo \
    RUSTUP_HOME=/home/runner/.rustup
ENV PATH=/home/runner/.cargo/bin:$PATH
RUN mkdir -p "$CARGO_HOME" "$RUSTUP_HOME" \
    && chown runner:runner "$CARGO_HOME" "$RUSTUP_HOME"

COPY --chmod=0755 entrypoint.sh /usr/local/bin/lamina-runner-entrypoint
USER runner
WORKDIR /home/runner
ENTRYPOINT ["/usr/local/bin/lamina-runner-entrypoint"]
