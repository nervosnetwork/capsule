.PHONY: build clean

TARGET ?= debug
SAVE_DIR ?= build

build:
	make -C deps/ckb-lua all-via-docker
	mkdir -p $(SAVE_DIR)/$(TARGET)
	cp -a deps/ckb-lua/build/. $(SAVE_DIR)/$(TARGET)

clean:
	make -C deps/ckb-lua clean
	rm -rf $(SAVE_DIR)/$(TARGET)
