use crate::semantic::Type;

pub(super) fn emit_allocation_runtime() -> String {
    r#"#ifndef MLG_ALLOCATION_FAIL_AFTER
#define MLG_ALLOCATION_FAIL_AFTER UINT64_MAX
#endif

static uint64_t mallang_allocation_attempts = 0;
static uint64_t mallang_live_allocations = 0;

static bool MLG_UNUSED mallang_should_fail_allocation(void) {
    bool mlg_should_fail = mallang_allocation_attempts == MLG_ALLOCATION_FAIL_AFTER;
    mallang_allocation_attempts = mallang_allocation_attempts + 1;
    return mlg_should_fail;
}

static void *MLG_UNUSED mallang_alloc(size_t mlg_size, const char *mlg_failure_message) {
    if (mallang_should_fail_allocation()) {
        mallang_runtime_error(mlg_failure_message);
    }
    void *mlg_allocation = malloc(mlg_size);
    if (mlg_allocation == NULL) {
        mallang_runtime_error(mlg_failure_message);
    }
    mallang_live_allocations = mallang_live_allocations + 1;
    return mlg_allocation;
}

static void *MLG_UNUSED mallang_realloc(
    void *mlg_allocation,
    size_t mlg_size,
    const char *mlg_failure_message
) {
    bool mlg_creates_allocation = mlg_allocation == NULL;
    if (mallang_should_fail_allocation()) {
        mallang_runtime_error(mlg_failure_message);
    }
    void *mlg_resized = realloc(mlg_allocation, mlg_size);
    if (mlg_resized == NULL) {
        mallang_runtime_error(mlg_failure_message);
    }
    if (mlg_creates_allocation) {
        mallang_live_allocations = mallang_live_allocations + 1;
    }
    return mlg_resized;
}

static void MLG_UNUSED mallang_dealloc(void *mlg_allocation) {
    if (mlg_allocation == NULL) {
        return;
    }
    if (mallang_live_allocations == 0) {
        mallang_runtime_error("allocation accounting underflow");
    }
    mallang_live_allocations = mallang_live_allocations - 1;
    free(mlg_allocation);
}

static uint64_t MLG_UNUSED mallang_allocation_attempt_count(void) {
    return mallang_allocation_attempts;
}

static uint64_t MLG_UNUSED mallang_live_allocation_count(void) {
    return mallang_live_allocations;
}

"#
    .to_string()
}

pub(super) fn emit_string_runtime(defined_types: &[Type]) -> String {
    if !defined_types.contains(&Type::String) {
        return String::new();
    }

    r#"enum {
    MLG_STRING_STATIC = 0,
    MLG_STRING_OWNED = 1
};

typedef struct {
    const char *mlg_data;
    size_t mlg_len;
    uint8_t mlg_storage;
} mlg_String;

static void MLG_UNUSED mallang_validate_string(mlg_String mlg_value) {
    if (mlg_value.mlg_storage != MLG_STRING_STATIC &&
        mlg_value.mlg_storage != MLG_STRING_OWNED) {
        mallang_runtime_error("invalid string storage");
    }
    if (mlg_value.mlg_data == NULL) {
        mallang_runtime_error("invalid string data");
    }
}

static mlg_String MLG_UNUSED mallang_string_owned_copy(mlg_String mlg_source) {
    mallang_validate_string(mlg_source);
    if (mlg_source.mlg_len == SIZE_MAX) {
        mallang_runtime_error("string allocation size overflow");
    }
    char *mlg_data = mallang_alloc(
        mlg_source.mlg_len + 1,
        "string allocation failed"
    );
    if (mlg_source.mlg_len > 0) {
        memcpy(mlg_data, mlg_source.mlg_data, mlg_source.mlg_len);
    }
    mlg_data[mlg_source.mlg_len] = '\0';
    return (mlg_String){
        .mlg_data = mlg_data,
        .mlg_len = mlg_source.mlg_len,
        .mlg_storage = MLG_STRING_OWNED
    };
}

static bool MLG_UNUSED mallang_string_equal(mlg_String mlg_left, mlg_String mlg_right) {
    mallang_validate_string(mlg_left);
    mallang_validate_string(mlg_right);
    return mlg_left.mlg_len == mlg_right.mlg_len &&
        (mlg_left.mlg_len == 0 ||
         memcmp(mlg_left.mlg_data, mlg_right.mlg_data, mlg_left.mlg_len) == 0);
}

static void MLG_UNUSED mallang_print_string(mlg_String mlg_value) {
    mallang_validate_string(mlg_value);
    if (mlg_value.mlg_len > 0) {
        (void)fwrite(mlg_value.mlg_data, 1, mlg_value.mlg_len, stdout);
    }
}

"#
    .to_string()
}
