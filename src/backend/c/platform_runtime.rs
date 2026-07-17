use std::collections::BTreeSet;

use crate::{
    ast::ParamMode,
    ir::IrProgram,
    semantic::{FunctionParamType, FunctionType, Type},
    standard::{StandardIntrinsic, StandardType},
};

use super::{
    names::{c_field, TypeCName},
    CompileError,
};

pub(super) fn emit_platform_runtime(
    program: &IrProgram,
    used: &BTreeSet<StandardIntrinsic>,
    for_each_line_uses: &[(String, Type, Type)],
) -> Result<String, CompileError> {
    let platform_intrinsics = [
        StandardIntrinsic::FsReadText,
        StandardIntrinsic::FsWriteText,
        StandardIntrinsic::FsForEachLine,
        StandardIntrinsic::IoReadStdin,
        StandardIntrinsic::IoWriteStdout,
        StandardIntrinsic::IoWriteStderr,
        StandardIntrinsic::OsArgs,
        StandardIntrinsic::OsEnv,
        StandardIntrinsic::OsExit,
    ];
    if !platform_intrinsics
        .iter()
        .any(|intrinsic| used.contains(intrinsic))
    {
        return Ok(String::new());
    }

    let layout = StandardErrorLayout::new(program)?;
    let mut output = emit_error_runtime(&layout);
    if used.contains(&StandardIntrinsic::FsReadText)
        || used.contains(&StandardIntrinsic::FsWriteText)
        || used.contains(&StandardIntrinsic::FsForEachLine)
    {
        output.push_str(FILE_PATH_RUNTIME);
    }
    if used.contains(&StandardIntrinsic::FsReadText) {
        output.push_str(&emit_fs_read_runtime(&layout));
    }
    if used.contains(&StandardIntrinsic::FsWriteText) {
        output.push_str(&emit_fs_write_runtime(&layout));
    }
    for (helper, context, state) in for_each_line_uses {
        output.push_str(&emit_fs_for_each_line_runtime(
            &layout, helper, context, state,
        ));
    }
    if used.contains(&StandardIntrinsic::OsArgs) {
        output.push_str(&emit_os_args_runtime(&layout));
    }
    if used.contains(&StandardIntrinsic::OsEnv) {
        output.push_str(&emit_os_env_runtime(&layout));
    }
    if used.contains(&StandardIntrinsic::OsExit) {
        output.push_str(OS_EXIT_RUNTIME);
    }
    if used.contains(&StandardIntrinsic::IoReadStdin) {
        output.push_str(&emit_io_read_runtime(&layout));
    }
    if used.contains(&StandardIntrinsic::IoWriteStdout)
        || used.contains(&StandardIntrinsic::IoWriteStderr)
    {
        output.push_str(&emit_io_write_runtime(&layout));
    }
    if used.contains(&StandardIntrinsic::IoWriteStdout) {
        output.push_str(&emit_io_write_wrapper(
            &layout,
            "mallang_std_io_write_stdout",
            "stdout",
            "standard output write failed",
        ));
    }
    if used.contains(&StandardIntrinsic::IoWriteStderr) {
        output.push_str(&emit_io_write_wrapper(
            &layout,
            "mallang_std_io_write_stderr",
            "stderr",
            "standard error write failed",
        ));
    }
    Ok(output)
}

struct StandardErrorLayout {
    error: Type,
    kind: Type,
    not_found: usize,
    permission_denied: usize,
    already_exists: usize,
    invalid_input: usize,
    invalid_data: usize,
    interrupted: usize,
    other: usize,
}

impl StandardErrorLayout {
    fn new(program: &IrProgram) -> Result<Self, CompileError> {
        let error = program
            .structs
            .iter()
            .find(|declaration| declaration.intrinsic == Some(StandardType::Error))
            .ok_or_else(|| {
                CompileError::new("IR invariant violation: standard Error type is missing")
            })?;
        let kind = program
            .enums
            .iter()
            .find(|declaration| declaration.intrinsic == Some(StandardType::ErrorKind))
            .ok_or_else(|| {
                CompileError::new("IR invariant violation: standard errors.Kind type is missing")
            })?;
        let tag = |name: &str| {
            kind.variants
                .iter()
                .position(|variant| variant.name == name)
                .ok_or_else(|| {
                    CompileError::new(format!(
                        "IR invariant violation: errors.Kind.{name} is missing"
                    ))
                })
        };
        Ok(Self {
            error: Type::Struct(error.name.clone()),
            kind: Type::Enum(kind.name.clone()),
            not_found: tag("NotFound")?,
            permission_denied: tag("PermissionDenied")?,
            already_exists: tag("AlreadyExists")?,
            invalid_input: tag("InvalidInput")?,
            invalid_data: tag("InvalidData")?,
            interrupted: tag("Interrupted")?,
            other: tag("Other")?,
        })
    }

    fn result(&self, ok: Type) -> Type {
        Type::Result(Box::new(ok), Box::new(self.error.clone()))
    }
}

fn emit_error_runtime(layout: &StandardErrorLayout) -> String {
    render(
        ERROR_RUNTIME,
        &[
            ("<ERROR_TYPE>", layout.error.c_name()),
            ("<KIND_TYPE>", layout.kind.c_name()),
            ("<NOT_FOUND_TAG>", layout.not_found.to_string()),
            (
                "<PERMISSION_DENIED_TAG>",
                layout.permission_denied.to_string(),
            ),
            ("<ALREADY_EXISTS_TAG>", layout.already_exists.to_string()),
            ("<INVALID_INPUT_TAG>", layout.invalid_input.to_string()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<INTERRUPTED_TAG>", layout.interrupted.to_string()),
            ("<OTHER_TAG>", layout.other.to_string()),
            ("<FIELD_KIND>", c_field("kind")),
            ("<FIELD_MESSAGE>", c_field("message")),
        ],
    )
}

fn emit_os_args_runtime(layout: &StandardErrorLayout) -> String {
    let slice = Type::Slice(Box::new(Type::String));
    render(
        OS_ARGS_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(slice.clone()).c_name()),
            ("<SLICE_TYPE>", slice.c_name()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_OK>", c_field("Ok")),
            ("<FIELD_ERR>", c_field("Err")),
            ("<FIELD_DATA>", c_field("data")),
            ("<FIELD_LEN>", c_field("len")),
            ("<FIELD_CAP>", c_field("cap")),
        ],
    )
}

fn emit_fs_read_runtime(layout: &StandardErrorLayout) -> String {
    render(
        FS_READ_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(Type::String).c_name()),
            ("<INVALID_INPUT_TAG>", layout.invalid_input.to_string()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<OTHER_TAG>", layout.other.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_OK>", c_field("Ok")),
            ("<FIELD_ERR>", c_field("Err")),
        ],
    )
}

fn emit_fs_write_runtime(layout: &StandardErrorLayout) -> String {
    render(
        FS_WRITE_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(Type::Unit).c_name()),
            ("<INVALID_INPUT_TAG>", layout.invalid_input.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_ERR>", c_field("Err")),
        ],
    )
}

fn emit_fs_for_each_line_runtime(
    layout: &StandardErrorLayout,
    helper: &str,
    context: &Type,
    state: &Type,
) -> String {
    let callback = Type::Function(FunctionType {
        mutable: false,
        params: vec![
            FunctionParamType {
                mode: ParamMode::Con,
                ty: context.clone(),
            },
            FunctionParamType {
                mode: ParamMode::Mut,
                ty: state.clone(),
            },
            FunctionParamType {
                mode: ParamMode::Owned,
                ty: Type::Int,
            },
            FunctionParamType {
                mode: ParamMode::Con,
                ty: Type::String,
            },
        ],
        return_type: Box::new(Type::Unit),
    });
    render(
        FS_FOR_EACH_LINE_RUNTIME,
        &[
            ("<HELPER>", helper.to_string()),
            ("<RESULT_TYPE>", layout.result(Type::Unit).c_name()),
            ("<CONTEXT_PARAM>", context.c_param_type(ParamMode::Con)),
            ("<STATE_PARAM>", state.c_param_type(ParamMode::Mut)),
            ("<CALLBACK_TYPE>", callback.c_name()),
            ("<INVALID_INPUT_TAG>", layout.invalid_input.to_string()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<OTHER_TAG>", layout.other.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_ERR>", c_field("Err")),
        ],
    )
}

fn emit_os_env_runtime(layout: &StandardErrorLayout) -> String {
    let option = Type::Option(Box::new(Type::String));
    render(
        OS_ENV_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(option.clone()).c_name()),
            ("<OPTION_TYPE>", option.c_name()),
            ("<INVALID_INPUT_TAG>", layout.invalid_input.to_string()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_OK>", c_field("Ok")),
            ("<FIELD_ERR>", c_field("Err")),
            ("<FIELD_SOME>", c_field("Some")),
        ],
    )
}

fn emit_io_read_runtime(layout: &StandardErrorLayout) -> String {
    render(
        IO_READ_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(Type::String).c_name()),
            ("<INVALID_DATA_TAG>", layout.invalid_data.to_string()),
            ("<OTHER_TAG>", layout.other.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_OK>", c_field("Ok")),
            ("<FIELD_ERR>", c_field("Err")),
        ],
    )
}

fn emit_io_write_runtime(layout: &StandardErrorLayout) -> String {
    render(
        IO_WRITE_RUNTIME,
        &[
            ("<RESULT_TYPE>", layout.result(Type::Unit).c_name()),
            ("<OTHER_TAG>", layout.other.to_string()),
            ("<FIELD_PAYLOAD>", c_field("payload")),
            ("<FIELD_ERR>", c_field("Err")),
        ],
    )
}

fn emit_io_write_wrapper(
    layout: &StandardErrorLayout,
    helper: &str,
    stream: &str,
    message: &str,
) -> String {
    format!(
        "static {} MLG_UNUSED {helper}(const mlg_String *mlg_text) {{\n    return mallang_std_io_write({stream}, mlg_text, \"{message}\");\n}}\n\n",
        layout.result(Type::Unit).c_name()
    )
}

fn render(template: &str, replacements: &[(&str, String)]) -> String {
    replacements
        .iter()
        .fold(template.to_string(), |rendered, (key, value)| {
            rendered.replace(key, value)
        })
}

const ERROR_RUNTIME: &str = r#"static <ERROR_TYPE> MLG_UNUSED mallang_std_error(
    int32_t mlg_kind,
    const char *mlg_message
) {
    return (<ERROR_TYPE>){
        .<FIELD_KIND> = (<KIND_TYPE>){ .tag = mlg_kind },
        .<FIELD_MESSAGE> = mallang_string_owned_from_bytes(
            mlg_message,
            strlen(mlg_message)
        )
    };
}

static int32_t MLG_UNUSED mallang_std_error_kind_from_errno(int mlg_error_number) {
#if defined(ENOENT)
    if (mlg_error_number == ENOENT) {
        return <NOT_FOUND_TAG>;
    }
#endif
#if defined(EACCES)
    if (mlg_error_number == EACCES) {
        return <PERMISSION_DENIED_TAG>;
    }
#endif
#if defined(EPERM)
    if (mlg_error_number == EPERM) {
        return <PERMISSION_DENIED_TAG>;
    }
#endif
#if defined(EEXIST)
    if (mlg_error_number == EEXIST) {
        return <ALREADY_EXISTS_TAG>;
    }
#endif
#if defined(EINVAL)
    if (mlg_error_number == EINVAL) {
        return <INVALID_INPUT_TAG>;
    }
#endif
#if defined(EILSEQ)
    if (mlg_error_number == EILSEQ) {
        return <INVALID_DATA_TAG>;
    }
#endif
#if defined(EINTR)
    if (mlg_error_number == EINTR) {
        return <INTERRUPTED_TAG>;
    }
#endif
    return <OTHER_TAG>;
}

static <ERROR_TYPE> MLG_UNUSED mallang_std_errno_error(
    int mlg_error_number,
    const char *mlg_message
) {
    return mallang_std_error(
        mallang_std_error_kind_from_errno(mlg_error_number),
        mlg_message
    );
}

"#;

const FILE_PATH_RUNTIME: &str = r#"static bool MLG_UNUSED mallang_std_file_path(
    const mlg_String *mlg_path,
    char **mlg_path_out
) {
    mallang_validate_string(*mlg_path);
    if (memchr(mlg_path->mlg_data, '\0', mlg_path->mlg_len) != NULL) {
        return false;
    }
    if (mlg_path->mlg_len == SIZE_MAX) {
        mallang_runtime_error("file path allocation size overflow");
    }
    char *mlg_path_data = mallang_alloc(
        mlg_path->mlg_len + 1,
        "file path allocation failed"
    );
    if (mlg_path->mlg_len > 0) {
        memcpy(mlg_path_data, mlg_path->mlg_data, mlg_path->mlg_len);
    }
    mlg_path_data[mlg_path->mlg_len] = '\0';
    *mlg_path_out = mlg_path_data;
    return true;
}

"#;

const FS_FOR_EACH_LINE_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED <HELPER>(
    const mlg_String *mlg_path,
    <CONTEXT_PARAM> mlg_context,
    <STATE_PARAM> mlg_state,
    const <CALLBACK_TYPE> *mlg_visit
) {
    if (mlg_visit == NULL || mlg_visit->mlg_call == NULL || mlg_state == NULL) {
        mallang_runtime_error("invalid file line visitor");
    }

    char *mlg_path_data = NULL;
    if (!mallang_std_file_path(mlg_path, &mlg_path_data)) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_INPUT_TAG>,
                    "file path contains NUL"
                )
            }
        };
    }

    errno = 0;
    FILE *mlg_file = fopen(mlg_path_data, "rb");
    int mlg_open_error = errno;
    mallang_dealloc(mlg_path_data);
    if (mlg_file == NULL) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_open_error,
                    "file open failed"
                )
            }
        };
    }

    size_t mlg_line_cap = 256;
    char *mlg_line_data = mallang_alloc(
        mlg_line_cap,
        "file line allocation failed"
    );
    unsigned char mlg_read_buffer[8192];
    size_t mlg_read_pos = 0;
    size_t mlg_read_len = 0;
    bool mlg_pending_read_failure = false;
    int mlg_pending_read_error = 0;
    bool mlg_eof = false;
    int64_t mlg_line_number = 0;
    int32_t mlg_failure_kind = <OTHER_TAG>;
    int mlg_failure_errno = 0;
    const char *mlg_failure_message = "file line read failed";

    while (!mlg_eof) {
        size_t mlg_line_len = 0;
        bool mlg_has_line = false;

        while (true) {
            if (mlg_read_pos == mlg_read_len) {
                if (mlg_pending_read_failure) {
                    mlg_failure_errno = mlg_pending_read_error;
                    goto mlg_file_line_failure;
                }
                errno = 0;
                mlg_read_len = fread(
                    mlg_read_buffer,
                    1,
                    sizeof(mlg_read_buffer),
                    mlg_file
                );
                int mlg_read_error = errno;
                mlg_read_pos = 0;
                if (mlg_read_len == 0) {
                    if (ferror(mlg_file) || mlg_read_error != 0) {
                        mlg_failure_errno = mlg_read_error;
                        goto mlg_file_line_failure;
                    }
                    if (feof(mlg_file)) {
                        mlg_eof = true;
                        break;
                    }
                    mlg_failure_message = "file line read made no progress";
                    goto mlg_file_line_failure;
                }
                if (ferror(mlg_file)) {
                    mlg_pending_read_failure = true;
                    mlg_pending_read_error = mlg_read_error;
                }
            }

            unsigned char mlg_byte = mlg_read_buffer[mlg_read_pos];
            mlg_read_pos = mlg_read_pos + 1;
            mlg_has_line = true;
            if (mlg_byte == '\n') {
                break;
            }
            if (mlg_line_len == mlg_line_cap - 1) {
                if (mlg_line_cap > SIZE_MAX / 2) {
                    mallang_runtime_error("file line allocation size overflow");
                }
                mlg_line_cap = mlg_line_cap * 2;
                mlg_line_data = mallang_realloc(
                    mlg_line_data,
                    mlg_line_cap,
                    "file line allocation failed"
                );
            }
            mlg_line_data[mlg_line_len] = (char)mlg_byte;
            mlg_line_len = mlg_line_len + 1;
            if (mlg_line_len > INT64_MAX) {
                mallang_runtime_error("file line exceeds string size limit");
            }
        }

        if (!mlg_has_line) {
            break;
        }
        if (!mallang_utf8_scalar_count_bytes(mlg_line_data, mlg_line_len, NULL)) {
            mlg_failure_kind = <INVALID_DATA_TAG>;
            mlg_failure_message = "file line is not valid UTF-8";
            goto mlg_file_line_failure;
        }
        if (mlg_line_number == INT64_MAX) {
            mallang_runtime_error("file line count overflow");
        }
        mlg_line_number = mlg_line_number + 1;
        mlg_line_data[mlg_line_len] = '\0';
        mlg_String mlg_line = (mlg_String){
            .mlg_data = mlg_line_data,
            .mlg_len = mlg_line_len,
            .mlg_storage = MLG_STRING_STATIC
        };
        mlg_visit->mlg_call(
            mlg_visit->mlg_env,
            mlg_context,
            mlg_state,
            mlg_line_number,
            &mlg_line
        );
    }

    mallang_dealloc(mlg_line_data);
    errno = 0;
    if (fclose(mlg_file) != 0) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    errno,
                    "file close failed"
                )
            }
        };
    }
    return (<RESULT_TYPE>){ .tag = 0 };

mlg_file_line_failure:
    mallang_dealloc(mlg_line_data);
    errno = 0;
    (void)fclose(mlg_file);
    if (mlg_failure_errno != 0) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_failure_errno,
                    mlg_failure_message
                )
            }
        };
    }
    return (<RESULT_TYPE>){
        .tag = 1,
        .<FIELD_PAYLOAD> = {
            .<FIELD_ERR> = mallang_std_error(
                mlg_failure_kind,
                mlg_failure_message
            )
        }
    };
}

"#;

const FS_READ_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED mallang_std_fs_read_text(
    const mlg_String *mlg_path
) {
    char *mlg_path_data = NULL;
    if (!mallang_std_file_path(mlg_path, &mlg_path_data)) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_INPUT_TAG>,
                    "file path contains NUL"
                )
            }
        };
    }

    errno = 0;
    FILE *mlg_file = fopen(mlg_path_data, "rb");
    int mlg_open_error = errno;
    mallang_dealloc(mlg_path_data);
    if (mlg_file == NULL) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_open_error,
                    "file open failed"
                )
            }
        };
    }

    size_t mlg_len = 0;
    size_t mlg_cap = 4096;
    char *mlg_data = mallang_alloc(mlg_cap, "file read allocation failed");
    while (true) {
        if (mlg_len == mlg_cap) {
            if (mlg_cap > SIZE_MAX / 2) {
                mallang_runtime_error("file read allocation size overflow");
            }
            mlg_cap = mlg_cap * 2;
            mlg_data = mallang_realloc(
                mlg_data,
                mlg_cap,
                "file read allocation failed"
            );
        }

        errno = 0;
        size_t mlg_read = fread(mlg_data + mlg_len, 1, mlg_cap - mlg_len, mlg_file);
        int mlg_read_error = errno;
        mlg_len = mlg_len + mlg_read;
        if (mlg_len > INT64_MAX) {
            mallang_dealloc(mlg_data);
            mallang_runtime_error("file content exceeds string size limit");
        }
        if (mlg_read > 0) {
            continue;
        }
        if (ferror(mlg_file) || mlg_read_error != 0) {
            mallang_dealloc(mlg_data);
            errno = 0;
            int mlg_close_status = fclose(mlg_file);
            if (mlg_read_error == 0 && mlg_close_status != 0) {
                mlg_read_error = errno;
            }
            return (<RESULT_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_errno_error(
                        mlg_read_error,
                        "file read failed"
                    )
                }
            };
        }
        if (feof(mlg_file)) {
            break;
        }
        mallang_dealloc(mlg_data);
        (void)fclose(mlg_file);
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <OTHER_TAG>,
                    "file read made no progress"
                )
            }
        };
    }

    errno = 0;
    if (fclose(mlg_file) != 0) {
        int mlg_close_error = errno;
        mallang_dealloc(mlg_data);
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_close_error,
                    "file close failed"
                )
            }
        };
    }
    if (!mallang_utf8_scalar_count_bytes(mlg_data, mlg_len, NULL)) {
        mallang_dealloc(mlg_data);
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_DATA_TAG>,
                    "file content is not valid UTF-8"
                )
            }
        };
    }
    mlg_data[mlg_len] = '\0';
    return (<RESULT_TYPE>){
        .tag = 0,
        .<FIELD_PAYLOAD> = {
            .<FIELD_OK> = (mlg_String){
                .mlg_data = mlg_data,
                .mlg_len = mlg_len,
                .mlg_storage = MLG_STRING_OWNED
            }
        }
    };
}

"#;

const FS_WRITE_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED mallang_std_fs_write_text(
    const mlg_String *mlg_path,
    const mlg_String *mlg_text
) {
    mallang_validate_string(*mlg_text);
    char *mlg_path_data = NULL;
    if (!mallang_std_file_path(mlg_path, &mlg_path_data)) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_INPUT_TAG>,
                    "file path contains NUL"
                )
            }
        };
    }

    errno = 0;
    FILE *mlg_file = fopen(mlg_path_data, "wb");
    int mlg_open_error = errno;
    mallang_dealloc(mlg_path_data);
    if (mlg_file == NULL) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_open_error,
                    "file open failed"
                )
            }
        };
    }

    size_t mlg_written = 0;
    while (mlg_written < mlg_text->mlg_len) {
        errno = 0;
        size_t mlg_step = fwrite(
            mlg_text->mlg_data + mlg_written,
            1,
            mlg_text->mlg_len - mlg_written,
            mlg_file
        );
        int mlg_write_error = errno;
        mlg_written = mlg_written + mlg_step;
        if (mlg_step > 0) {
            continue;
        }
        errno = 0;
        int mlg_close_status = fclose(mlg_file);
        if (mlg_write_error == 0 && mlg_close_status != 0) {
            mlg_write_error = errno;
        }
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_write_error,
                    "file write failed"
                )
            }
        };
    }

    errno = 0;
    if (fclose(mlg_file) != 0) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    errno,
                    "file close failed"
                )
            }
        };
    }
    return (<RESULT_TYPE>){ .tag = 0 };
}

"#;

const OS_ARGS_RUNTIME: &str = r#"static int mallang_process_argc = 0;
static char **mallang_process_argv = NULL;

static void MLG_UNUSED mallang_process_init(int mlg_argc, char **mlg_argv) {
    if (mlg_argc < 0 || (mlg_argc > 0 && mlg_argv == NULL)) {
        mallang_runtime_error("invalid process argument storage");
    }
    mallang_process_argc = mlg_argc;
    mallang_process_argv = mlg_argv;
}

static void MLG_UNUSED mallang_std_drop_partial_strings(
    mlg_String *mlg_values,
    size_t mlg_initialized
) {
    for (size_t mlg_index = 0; mlg_index < mlg_initialized; mlg_index = mlg_index + 1) {
        mallang_validate_string(mlg_values[mlg_index]);
        mallang_dealloc((void *)mlg_values[mlg_index].mlg_data);
    }
    mallang_dealloc(mlg_values);
}

static <RESULT_TYPE> MLG_UNUSED mallang_std_os_args(void) {
    size_t mlg_count = (size_t)mallang_process_argc;
    if (mlg_count > SIZE_MAX / sizeof(mlg_String)) {
        mallang_runtime_error("process argument allocation size overflow");
    }
    mlg_String *mlg_values = NULL;
    if (mlg_count > 0) {
        mlg_values = mallang_alloc(
            mlg_count * sizeof(mlg_String),
            "process argument allocation failed"
        );
    }

    for (size_t mlg_index = 0; mlg_index < mlg_count; mlg_index = mlg_index + 1) {
        const char *mlg_argument = mallang_process_argv[mlg_index];
        if (mlg_argument == NULL) {
            mallang_std_drop_partial_strings(mlg_values, mlg_index);
            return (<RESULT_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_error(
                        <INVALID_DATA_TAG>,
                        "invalid process argument"
                    )
                }
            };
        }
        size_t mlg_len = strlen(mlg_argument);
        if (mlg_len > INT64_MAX ||
            !mallang_utf8_scalar_count_bytes(mlg_argument, mlg_len, NULL)) {
            mallang_std_drop_partial_strings(mlg_values, mlg_index);
            return (<RESULT_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_error(
                        <INVALID_DATA_TAG>,
                        "process argument is not valid UTF-8"
                    )
                }
            };
        }
        mlg_values[mlg_index] = mallang_string_owned_from_bytes(mlg_argument, mlg_len);
    }

    return (<RESULT_TYPE>){
        .tag = 0,
        .<FIELD_PAYLOAD> = {
            .<FIELD_OK> = (<SLICE_TYPE>){
                .<FIELD_DATA> = mlg_values,
                .<FIELD_LEN> = (int64_t)mlg_count,
                .<FIELD_CAP> = (int64_t)mlg_count
            }
        }
    };
}

"#;

const OS_ENV_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED mallang_std_os_env(
    const mlg_String *mlg_name
) {
    mallang_validate_string(*mlg_name);
    if (memchr(mlg_name->mlg_data, '\0', mlg_name->mlg_len) != NULL) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_INPUT_TAG>,
                    "environment name contains NUL"
                )
            }
        };
    }
    if (mlg_name->mlg_len == SIZE_MAX) {
        mallang_runtime_error("environment name allocation size overflow");
    }
    char *mlg_name_data = mallang_alloc(
        mlg_name->mlg_len + 1,
        "environment name allocation failed"
    );
    if (mlg_name->mlg_len > 0) {
        memcpy(mlg_name_data, mlg_name->mlg_data, mlg_name->mlg_len);
    }
    mlg_name_data[mlg_name->mlg_len] = '\0';
    const char *mlg_value = getenv(mlg_name_data);
    mallang_dealloc(mlg_name_data);

    if (mlg_value == NULL) {
        return (<RESULT_TYPE>){
            .tag = 0,
            .<FIELD_PAYLOAD> = {
                .<FIELD_OK> = (<OPTION_TYPE>){ .tag = 0 }
            }
        };
    }

    size_t mlg_len = strlen(mlg_value);
    if (mlg_len > INT64_MAX ||
        !mallang_utf8_scalar_count_bytes(mlg_value, mlg_len, NULL)) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_DATA_TAG>,
                    "environment value is not valid UTF-8"
                )
            }
        };
    }

    return (<RESULT_TYPE>){
        .tag = 0,
        .<FIELD_PAYLOAD> = {
            .<FIELD_OK> = (<OPTION_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_SOME> = mallang_string_owned_from_bytes(mlg_value, mlg_len)
                }
            }
        }
    };
}

"#;

const OS_EXIT_RUNTIME: &str = r#"static _Noreturn void MLG_UNUSED mallang_std_os_exit(
    int64_t mlg_code
) {
    if (mlg_code < 0 || mlg_code > 255) {
        mallang_runtime_error("process exit code out of range");
    }
    exit((int)mlg_code);
}

"#;

const IO_READ_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED mallang_std_io_read_stdin(void) {
    size_t mlg_len = 0;
    size_t mlg_cap = 4096;
    char *mlg_data = mallang_alloc(mlg_cap, "standard input allocation failed");

    while (true) {
        if (mlg_len == mlg_cap) {
            if (mlg_cap > SIZE_MAX / 2) {
                mallang_runtime_error("standard input allocation size overflow");
            }
            mlg_cap = mlg_cap * 2;
            mlg_data = mallang_realloc(
                mlg_data,
                mlg_cap,
                "standard input allocation failed"
            );
        }

        errno = 0;
        size_t mlg_read = fread(mlg_data + mlg_len, 1, mlg_cap - mlg_len, stdin);
        mlg_len = mlg_len + mlg_read;
        if (mlg_len > INT64_MAX) {
            mallang_dealloc(mlg_data);
            mallang_runtime_error("standard input exceeds string size limit");
        }
        if (mlg_read > 0) {
            continue;
        }
        if (ferror(stdin)) {
            int mlg_error_number = errno;
            mallang_dealloc(mlg_data);
            return (<RESULT_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_errno_error(
                        mlg_error_number,
                        "standard input read failed"
                    )
                }
            };
        }
        if (feof(stdin)) {
            break;
        }
        mallang_dealloc(mlg_data);
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <OTHER_TAG>,
                    "standard input read made no progress"
                )
            }
        };
    }

    if (!mallang_utf8_scalar_count_bytes(mlg_data, mlg_len, NULL)) {
        mallang_dealloc(mlg_data);
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_error(
                    <INVALID_DATA_TAG>,
                    "standard input is not valid UTF-8"
                )
            }
        };
    }
    mlg_data[mlg_len] = '\0';
    return (<RESULT_TYPE>){
        .tag = 0,
        .<FIELD_PAYLOAD> = {
            .<FIELD_OK> = (mlg_String){
                .mlg_data = mlg_data,
                .mlg_len = mlg_len,
                .mlg_storage = MLG_STRING_OWNED
            }
        }
    };
}

"#;

const IO_WRITE_RUNTIME: &str = r#"static <RESULT_TYPE> MLG_UNUSED mallang_std_io_write(
    FILE *mlg_stream,
    const mlg_String *mlg_text,
    const char *mlg_failure_message
) {
    mallang_validate_string(*mlg_text);
    size_t mlg_written = 0;
    while (mlg_written < mlg_text->mlg_len) {
        errno = 0;
        size_t mlg_step = fwrite(
            mlg_text->mlg_data + mlg_written,
            1,
            mlg_text->mlg_len - mlg_written,
            mlg_stream
        );
        mlg_written = mlg_written + mlg_step;
        if (mlg_step > 0) {
            continue;
        }
        int mlg_error_number = errno;
        if (!ferror(mlg_stream)) {
            return (<RESULT_TYPE>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_error(
                        <OTHER_TAG>,
                        "stream write made no progress"
                    )
                }
            };
        }
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(
                    mlg_error_number,
                    mlg_failure_message
                )
            }
        };
    }

    errno = 0;
    if (fflush(mlg_stream) != 0) {
        return (<RESULT_TYPE>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_errno_error(errno, mlg_failure_message)
            }
        };
    }
    return (<RESULT_TYPE>){ .tag = 0 };
}

"#;
