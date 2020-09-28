/* This example demostrate how to write a shared library */
#include "blockchain.h"
#include "ckb_syscalls.h"
#include "stdio.h"

#define SCRIPT_SIZE 32768

// Common error codes that might be returned by the script.
#define ERROR_ARGUMENTS_LEN -1
#define ERROR_ENCODING -2
#define ERROR_SYSCALL -3
#define ERROR_SCRIPT_TOO_LONG -21

__attribute__((visibility("default"))) uint32_t plus_42(uint32_t num) {
  return 42 + num;
}

__attribute__((visibility("default"))) char *foo() { return "foo"; }

__attribute__((visibility("default"))) uint64_t
read_args_len(uint64_t *args_len) {
  // Load current script
  unsigned char script[SCRIPT_SIZE];
  uint64_t len = SCRIPT_SIZE;
  int ret = ckb_load_script(script, &len, 0);
  if (ret != CKB_SUCCESS) {
    return ERROR_SYSCALL;
  }
  if (len > SCRIPT_SIZE) {
    return ERROR_SCRIPT_TOO_LONG;
  }
  mol_seg_t script_seg;
  script_seg.ptr = (uint8_t *)script;
  script_seg.size = len;

  // Verify data is a valid molecule structure
  if (MolReader_Script_verify(&script_seg, false) != MOL_OK) {
    return ERROR_ENCODING;
  }

  // Extract args from Script
  mol_seg_t args_seg = MolReader_Script_get_args(&script_seg);
  mol_seg_t args_bytes_seg = MolReader_Bytes_raw_bytes(&args_seg);
  // the printf only compiled under debug build
  printf("args length: %ld", args_bytes_seg.size);
  *args_len = args_bytes_seg.size;

  return 0;
}

