$(DEBUG_DIR)/{{name}}: src/{{name}}.c $(PROTOCOL_HEADER)
	$(CC) $(CFLAGS) $(DBGFLAGS) $(LDFLAGS) -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@

$(RELEASE_DIR)/{{name}}: src/{{name}}.c $(PROTOCOL_HEADER)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $<
	$(OBJCOPY) --only-keep-debug $@ $@.debug
	$(OBJCOPY) --strip-debug --strip-all $@
