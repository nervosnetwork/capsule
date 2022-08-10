FROM nervos/ckb-riscv-gnu-toolchain:bionic-20211214

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly-2022-08-01 -y
ENV PATH=/root/.cargo/bin:$PATH
# Install RISC-V target
RUN rustup target add riscv64imac-unknown-none-elf
# Install CKB binary patcher
RUN cargo install --git https://github.com/nervosnetwork/ckb-binary-patcher.git --rev b9489de4b3b9d59bc29bce945279bc6f28413113
# Install CKB debugger
RUN git clone https://github.com/nervosnetwork/ckb-standalone-debugger.git \
    && cd ckb-standalone-debugger && git checkout dd7f80a557875787964e09126d3b122146e9e88a && cd - \
    && cargo install --path ckb-standalone-debugger/bins --locked \
    && rm -r ckb-standalone-debugger
