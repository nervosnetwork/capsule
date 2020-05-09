FROM nervos/ckb-riscv-gnu-toolchain@sha256:7b168b4b109a0f741078a71b7c4dddaf1d283a5244608f7851f5714fbad273ba

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2020-04-23 -y
ENV PATH=/root/.cargo/bin:$PATH
# Install RISC-V target
RUN rustup target add riscv64imac-unknown-none-elf
# Install CKB binary patcher
RUN cargo install --git https://github.com/xxuejie/ckb-binary-patcher.git
