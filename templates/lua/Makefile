.PHONY: build clean

BUILD_TARGET ?= debug
DIR = build/$(BUILD_TARGET)

build-common:
	make -C deps/ckb-lua all-via-docker
	mkdir -p $(DIR)
	cp -a deps/ckb-lua/build/. $(DIR)

clean:
	make -C deps/ckb-lua clean
	rm -rf $(DIR)
