
LUA_BUILDER_IMAGE := nervos/ckb-riscv-gnu-toolchain@sha256:aae8a3f79705f67d505d1f1d5ddc694a4fd537ed1c7e9622420a470d59ba2ec3
LUA_TARGET := riscv64-unknown-linux-gnu
LUA_CC := $(LUA_TARGET)-gcc
LUA_CFLAGS := -fPIC -O3 -fno-builtin -nostdinc -nostdlib -nostartfiles -fvisibility=hidden -fdata-sections -ffunction-sections -I deps/ckb-lua/lualib -I deps/ckb-lua/include/ckb-c-stdlib -I deps/ckb-lua/include/ckb-c-stdlib/libc -I deps/ckb-lua/include/ckb-c-stdlib/molecule -Wall -Werror -Wno-nonnull -Wno-nonnull-compare -Wno-unused-function -g
LUA_LDFLAGS := -nostdlib -nostartfiles -fno-builtin -Wl,-static -fdata-sections -ffunction-sections -Wl,--gc-sections

build-linked-binary: src/{{ name }}.c
	$(LUA_CC) $(LUA_CFLAGS) $(LUA_LDFLAGS) -o $(DIR)/{{ name }} $< $(shell $(LUA_CC) --print-search-dirs | sed -n '/install:/p' | sed 's/install:\s*//g')libgcc.a

build-linked-binary-via-docker:
	docker run --rm -v `pwd`:/code ${LUA_BUILDER_IMAGE} bash -c "cd /code && make build-linked-binary"

build: build-common build-linked-binary-via-docker
