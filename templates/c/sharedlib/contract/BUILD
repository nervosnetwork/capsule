$(DEBUG_DIR)/{{name}}.so: src/{{name}}.c $(PROTOCOL_HEADER)
	$(CC) $(CFLAGS) $(DBGFLAGS) $(LDFLAGS) -shared -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

$(RELEASE_DIR)/{{name}}.so: src/{{name}}.c $(PROTOCOL_HEADER)
	$(CC) $(CFLAGS) $(LDFLAGS) -shared -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@
