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

static size_t MLG_UNUSED mallang_utf8_sequence_length(
    const char *mlg_data,
    size_t mlg_remaining
) {
    if (mlg_remaining == 0) {
        return 0;
    }
    const unsigned char *mlg_bytes = (const unsigned char *)mlg_data;
    unsigned char mlg_first = mlg_bytes[0];
    if (mlg_first <= 0x7f) {
        return 1;
    }
    if (mlg_first >= 0xc2 && mlg_first <= 0xdf) {
        return mlg_remaining >= 2 &&
            mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0xbf ? 2 : 0;
    }
    if (mlg_first >= 0xe0 && mlg_first <= 0xef) {
        if (mlg_remaining < 3 ||
            mlg_bytes[2] < 0x80 || mlg_bytes[2] > 0xbf) {
            return 0;
        }
        bool mlg_second_valid =
            (mlg_first == 0xe0 && mlg_bytes[1] >= 0xa0 && mlg_bytes[1] <= 0xbf) ||
            (mlg_first >= 0xe1 && mlg_first <= 0xec &&
             mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0xbf) ||
            (mlg_first == 0xed && mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0x9f) ||
            (mlg_first >= 0xee && mlg_first <= 0xef &&
             mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0xbf);
        return mlg_second_valid ? 3 : 0;
    }
    if (mlg_first >= 0xf0 && mlg_first <= 0xf4) {
        if (mlg_remaining < 4 ||
            mlg_bytes[2] < 0x80 || mlg_bytes[2] > 0xbf ||
            mlg_bytes[3] < 0x80 || mlg_bytes[3] > 0xbf) {
            return 0;
        }
        bool mlg_second_valid =
            (mlg_first == 0xf0 && mlg_bytes[1] >= 0x90 && mlg_bytes[1] <= 0xbf) ||
            (mlg_first >= 0xf1 && mlg_first <= 0xf3 &&
             mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0xbf) ||
            (mlg_first == 0xf4 && mlg_bytes[1] >= 0x80 && mlg_bytes[1] <= 0x8f);
        return mlg_second_valid ? 4 : 0;
    }
    return 0;
}

static bool MLG_UNUSED mallang_utf8_scalar_count_bytes(
    const char *mlg_data,
    size_t mlg_len,
    int64_t *mlg_count_out
) {
    size_t mlg_cursor = 0;
    int64_t mlg_count = 0;
    while (mlg_cursor < mlg_len) {
        size_t mlg_width = mallang_utf8_sequence_length(
            mlg_data + mlg_cursor,
            mlg_len - mlg_cursor
        );
        if (mlg_width == 0 || mlg_count == INT64_MAX) {
            return false;
        }
        mlg_cursor = mlg_cursor + mlg_width;
        mlg_count = mlg_count + 1;
    }
    if (mlg_count_out != NULL) {
        *mlg_count_out = mlg_count;
    }
    return true;
}

static void MLG_UNUSED mallang_validate_string(mlg_String mlg_value) {
    if (mlg_value.mlg_storage != MLG_STRING_STATIC &&
        mlg_value.mlg_storage != MLG_STRING_OWNED) {
        mallang_runtime_error("invalid string storage");
    }
    if (mlg_value.mlg_data == NULL) {
        mallang_runtime_error("invalid string data");
    }
    if (mlg_value.mlg_len > INT64_MAX) {
        mallang_runtime_error("invalid string length");
    }
    if (!mallang_utf8_scalar_count_bytes(
            mlg_value.mlg_data,
            mlg_value.mlg_len,
            NULL)) {
        mallang_runtime_error("invalid UTF-8 string data");
    }
}

static mlg_String MLG_UNUSED mallang_string_owned_from_bytes(
    const char *mlg_source,
    size_t mlg_len
) {
    if (mlg_source == NULL) {
        mallang_runtime_error("invalid string copy source");
    }
    if (mlg_len == SIZE_MAX) {
        mallang_runtime_error("string allocation size overflow");
    }
    char *mlg_data = mallang_alloc(mlg_len + 1, "string allocation failed");
    if (mlg_len > 0) {
        memcpy(mlg_data, mlg_source, mlg_len);
    }
    mlg_data[mlg_len] = '\0';
    return (mlg_String){
        .mlg_data = mlg_data,
        .mlg_len = mlg_len,
        .mlg_storage = MLG_STRING_OWNED
    };
}

static mlg_String MLG_UNUSED mallang_string_owned_copy(mlg_String mlg_source) {
    if (mlg_source.mlg_len == SIZE_MAX) {
        mallang_runtime_error("string allocation size overflow");
    }
    mallang_validate_string(mlg_source);
    return mallang_string_owned_from_bytes(mlg_source.mlg_data, mlg_source.mlg_len);
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
