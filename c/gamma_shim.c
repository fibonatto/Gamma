#include <lean/lean.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>

extern void lean_initialize(void);
extern void lean_init_task_manager(void);

#ifndef LEAN_CHECKER_INIT_SYMBOL
#error "LEAN_CHECKER_INIT_SYMBOL must match Lake's generated module initializer"
#endif
extern lean_obj_res LEAN_CHECKER_INIT_SYMBOL(uint8_t builtin,
                                             lean_obj_arg world);
extern lean_obj_res gamma_inspect(lean_obj_arg path, lean_obj_arg modules,
                                  lean_obj_arg world);

static pthread_once_t init_once = PTHREAD_ONCE_INIT;
static int init_status = -1;

static void initialize_gamma(void) {
  lean_initialize();
  lean_init_task_manager();

  lean_obj_res result = LEAN_CHECKER_INIT_SYMBOL(1, lean_io_mk_world());
  if (!lean_io_result_is_ok(result)) {
    lean_io_result_show_error(result);
    lean_dec(result);
    init_status = 1;
    return;
  }
  lean_dec(result);
  lean_io_mark_end_initialization();
  init_status = 0;
}

/* Returns malloc-owned UTF-8 JSON. The caller releases it with gamma_free. */
int gamma_inspect_json(const char *path, const char *modules, char **json_out) {
  if (!path || !modules || !json_out)
    return 2;
  *json_out = NULL;
  pthread_once(&init_once, initialize_gamma);
  if (init_status != 0)
    return 3;

  lean_obj_res p = lean_mk_string(path);
  lean_obj_res m = lean_mk_string(modules);
  lean_obj_res result = gamma_inspect(p, m, lean_io_mk_world());
  if (!lean_io_result_is_ok(result)) {
    lean_io_result_show_error(result);
    lean_dec(result);
    return 4;
  }

  b_lean_obj_res value = lean_io_result_get_value(result);
  const char *text = lean_string_cstr(value);
  size_t len = strlen(text);
  char *copy = (char *)malloc(len + 1);
  if (!copy) {
    lean_dec(result);
    return 5;
  }
  memcpy(copy, text, len + 1);
  lean_dec(result);
  *json_out = copy;
  return 0;
}

void gamma_free(char *ptr) { free(ptr); }
