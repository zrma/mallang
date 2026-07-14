use crate::semantic::Type;

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
    char *mlg_data = malloc(mlg_source.mlg_len + 1);
    if (mlg_data == NULL) {
        mallang_runtime_error("string allocation failed");
    }
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
