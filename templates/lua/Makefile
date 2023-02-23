.PHONY: build clean

BUILD_TARGET ?= debug
DIR = build/$(BUILD_TARGET)

build-common:
	make -C deps/ckb-lua all-via-docker
	cp -r deps/ckb-lua/build/. $(DIR)

clean:
	make -C deps/ckb-lua clean
	rm -rf $(DIR)
