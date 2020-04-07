# Rust compile
RUST_TARGET := riscv64imac-unknown-none-elf
RUSTFLAGS := "-C link-arg=-s"
CONTRACTS := {{ name }}

default: build
setup: install-binary-patcher

# CKB binary patcher 
install-binary-patcher:
	which ckb-binary-patcher || \
		cargo install --force --git https://github.com/xxuejie/ckb-binary-patcher.git

# Docker
BUILDER_DOCKER := jjy0/ckb-riscv-rust-toolchain:2020-2-6

##@ Contracts
{{ name }}_path := contracts/{{name}}
{{ name }}_bin := contracts/{{name}}/target/${RUST_TARGET}/release/{{name}}

CONTRACTS_BIN := $({{name}}_bin)
# deps
build: ${CONTRACTS_BIN}
	cp $^ build/

build-via-docker:
	docker run -eOWNER=`id -u`:`id -g` --rm -v `pwd`:/code ${BUILDER_DOCKER} bash -c "cd /code && make build"

$({{name}}_bin):
	cd $({{name}}_path) && RUSTFLAGS=${RUSTFLAGS} cargo build --target ${RUST_TARGET} --release && chown -R $$OWNER target && cd - && \
		ckb-binary-patcher -i $({{name}}_bin) -o $({{name}}_bin)

clean-cargo:
	CURRENT_DIR=`pwd`; \
	for proj in ${CONTRACTS} ; do \
        cd $$CURRENT_DIR/$$proj && cargo clean ; \
    done

clean: clean-cargo
	rm build/* || true

.PHONY: clean ${CONTRACTS_BIN}
