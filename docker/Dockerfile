FROM nervos/ckb-riscv-gnu-toolchain@sha256:994fbf92e0372a28f7aec4ebea75315e3ebcf2ed7a5e95520aab903ae8cfd21c

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2021-08-16 -y
ENV PATH=/root/.cargo/bin:$PATH
# Install RISC-V target
RUN rustup target add riscv64imac-unknown-none-elf
# Install CKB binary patcher
RUN cargo install --git https://github.com/nervosnetwork/ckb-binary-patcher.git --rev 930f0b468a8f426ebb759d9da735ebaa1e2f98ba
# Install CKB debugger
RUN git clone https://github.com/nervosnetwork/ckb-standalone-debugger.git \
    && cd ckb-standalone-debugger && git checkout 0.20.0-rc3 && cd - \
    && cargo install --path ckb-standalone-debugger/bins \
    && rm -r ckb-standalone-debugger
