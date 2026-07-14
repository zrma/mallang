use crate::{
    ir::{
        IrEnum, IrExpr, IrExprKind, IrForInit, IrForPost, IrProgram, IrStmt, IrStmtKind, IrStruct,
    },
    semantic::Type,
};

use super::{
    names::{c_field, drop_fn_name, TypeCName},
    utils::push_indented_lines,
    CompileError,
};

pub(super) fn collect_defined_types(program: &IrProgram) -> Vec<Type> {
    TypeEmitter::new(program).collect_defined_types()
}

pub(super) fn emit_type_definitions(
    program: &IrProgram,
    defined_types: &[Type],
) -> Result<String, CompileError> {
    TypeEmitter::new(program).emit_type_definitions(defined_types)
}

pub(super) fn emit_drop_helpers(
    program: &IrProgram,
    defined_types: &[Type],
) -> Result<String, CompileError> {
    TypeEmitter::new(program).emit_drop_helpers(defined_types)
}

struct TypeEmitter<'a> {
    program: &'a IrProgram,
}

impl<'a> TypeEmitter<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    fn collect_defined_types(&self) -> Vec<Type> {
        let mut types = Vec::new();
        for struct_def in &self.program.structs {
            collect_type(&Type::Struct(struct_def.name.clone()), &mut types);
            for field in &struct_def.fields {
                collect_type(&field.ty, &mut types);
            }
        }
        for enum_def in &self.program.enums {
            collect_type(&Type::Enum(enum_def.name.clone()), &mut types);
            for variant in &enum_def.variants {
                if let Some(payload) = &variant.payload {
                    collect_type(payload, &mut types);
                }
            }
        }
        for function in &self.program.functions {
            collect_type(&function.return_type, &mut types);
            for param in &function.params {
                collect_type(&param.ty, &mut types);
            }
            for stmt in &function.body {
                self.collect_stmt_types(stmt, &mut types);
            }
        }
        for closure in &self.program.closures {
            collect_type(&closure.return_type, &mut types);
            for capture in &closure.captures {
                collect_type(&capture.ty, &mut types);
            }
            for param in &closure.params {
                collect_type(&param.ty, &mut types);
            }
            for stmt in &closure.body {
                self.collect_stmt_types(stmt, &mut types);
            }
        }
        types
    }

    fn emit_type_definitions(&self, defined_types: &[Type]) -> Result<String, CompileError> {
        let mut output = String::new();
        let mut emitted = Vec::new();
        for ty in defined_types {
            self.emit_type_def(ty, &mut emitted, &mut Vec::new(), &mut output)?;
        }
        Ok(output)
    }

    fn emit_drop_helpers(&self, defined_types: &[Type]) -> Result<String, CompileError> {
        let mut output = String::new();
        let mut emitted = Vec::new();
        for ty in defined_types {
            self.emit_drop_helper(ty, &mut emitted, &mut Vec::new(), &mut output)?;
        }
        Ok(output)
    }

    fn emit_type_def(
        &self,
        ty: &Type,
        emitted: &mut Vec<Type>,
        visiting: &mut Vec<Type>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        if emitted.contains(ty) || matches!(ty, Type::Int | Type::Bool | Type::String | Type::Unit)
        {
            return Ok(());
        }
        if visiting.contains(ty) {
            return Err(CompileError::new(format!(
                "recursive type definition involving `{}` is not supported in v0",
                ty.source_name()
            )));
        }

        visiting.push(ty.clone());
        match ty {
            Type::Option(inner) => {
                self.emit_type_def(inner, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_adt(ty)?);
                output.push('\n');
            }
            Type::Result(ok, err) => {
                self.emit_type_def(ok, emitted, visiting, output)?;
                self.emit_type_def(err, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_adt(ty)?);
                output.push('\n');
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                for field in &struct_def.fields {
                    self.emit_type_def(&field.ty, emitted, visiting, output)?;
                }
                output.push_str(&self.typedef_for_struct(struct_def));
                output.push('\n');
            }
            Type::Enum(name) => {
                let enum_def = self.enum_def(name)?;
                for variant in &enum_def.variants {
                    if let Some(payload) = &variant.payload {
                        self.emit_type_def(payload, emitted, visiting, output)?;
                    }
                }
                output.push_str(&self.typedef_for_enum(enum_def));
                output.push('\n');
            }
            Type::Array { .. } => {
                output.push_str(&self.typedef_for_array(ty)?);
                output.push('\n');
            }
            Type::Slice(element) => {
                self.emit_type_def(element, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_slice(ty)?);
                output.push('\n');
            }
            Type::Function(function) => {
                for param in &function.params {
                    self.emit_type_def(&param.ty, emitted, visiting, output)?;
                }
                self.emit_type_def(&function.return_type, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_function(ty)?);
                output.push('\n');
            }
            Type::Int | Type::Bool | Type::String | Type::Unit => {}
        }
        visiting.pop();
        emitted.push(ty.clone());
        Ok(())
    }

    fn struct_def(&self, name: &str) -> Result<&IrStruct, CompileError> {
        self.program
            .structs
            .iter()
            .find(|struct_def| struct_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown struct `{name}`"))
            })
    }

    fn enum_def(&self, name: &str) -> Result<&IrEnum, CompileError> {
        self.program
            .enums
            .iter()
            .find(|enum_def| enum_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown enum `{name}`"))
            })
    }

    fn collect_stmt_types(&self, stmt: &IrStmt, types: &mut Vec<Type>) {
        match &stmt.kind {
            IrStmtKind::Let { ty, expr, .. } => {
                collect_type(ty, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::Assign { expr, .. }
            | IrStmtKind::Return { expr }
            | IrStmtKind::Expr { expr } => self.collect_expr_types(expr, types),
            IrStmtKind::Break | IrStmtKind::Continue => {}
            IrStmtKind::FieldAssign { base, expr, .. } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::IndexAssign { base, index, expr } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(index, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.collect_expr_types(condition, types);
                for stmt in then_body {
                    self.collect_stmt_types(stmt, types);
                }
                for stmt in else_body {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::For {
                init,
                condition,
                post,
                body,
                cleanup,
            } => {
                if let Some(init) = init.as_deref() {
                    self.collect_for_init_types(init, types);
                }
                if let Some(condition) = condition.as_deref() {
                    self.collect_expr_types(condition, types);
                }
                if let Some(post) = post.as_deref() {
                    self.collect_for_post_types(post, types);
                }
                for stmt in body {
                    self.collect_stmt_types(stmt, types);
                }
                for stmt in cleanup {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::RangeFor {
                source,
                element_ty,
                body,
                ..
            } => {
                self.collect_expr_types(source, types);
                collect_type(element_ty, types);
                for stmt in body {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::Drop { expr } => self.collect_expr_types(expr, types),
            IrStmtKind::Match { scrutinee, arms } => {
                self.collect_expr_types(scrutinee, types);
                for arm in arms {
                    for stmt in &arm.body {
                        self.collect_stmt_types(stmt, types);
                    }
                }
            }
        }
    }

    fn collect_for_init_types(&self, init: &IrForInit, types: &mut Vec<Type>) {
        match init {
            IrForInit::Let { ty, expr, .. } => {
                collect_type(ty, types);
                self.collect_expr_types(expr, types);
            }
        }
    }

    fn collect_for_post_types(&self, post: &IrForPost, types: &mut Vec<Type>) {
        match post {
            IrForPost::Assign { target, expr } => {
                self.collect_expr_types(target, types);
                self.collect_expr_types(expr, types);
            }
        }
    }

    fn collect_expr_types(&self, expr: &IrExpr, types: &mut Vec<Type>) {
        collect_type(&expr.ty, types);
        match &expr.kind {
            IrExprKind::If {
                condition,
                then_branch,
                then_cleanup,
                else_branch,
                else_cleanup,
            } => {
                self.collect_expr_types(condition, types);
                self.collect_expr_types(then_branch, types);
                for stmt in then_cleanup {
                    self.collect_stmt_types(stmt, types);
                }
                self.collect_expr_types(else_branch, types);
                for stmt in else_cleanup {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrExprKind::AdtConstructor { payload, .. } => {
                if let Some(payload) = payload {
                    self.collect_expr_types(payload, types);
                }
            }
            IrExprKind::EnumConstructor { payload, .. } => {
                if let Some(payload) = payload {
                    self.collect_expr_types(payload, types);
                }
            }
            IrExprKind::Match { scrutinee, arms } => {
                self.collect_expr_types(scrutinee, types);
                for arm in arms {
                    self.collect_expr_types(&arm.expr, types);
                    for stmt in &arm.cleanup {
                        self.collect_stmt_types(stmt, types);
                    }
                }
            }
            IrExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_expr_types(&field.expr, types);
                }
            }
            IrExprKind::ArrayLiteral { elements } => {
                for element in elements {
                    self.collect_expr_types(element, types);
                }
            }
            IrExprKind::FieldAccess { base, .. } => self.collect_expr_types(base, types),
            IrExprKind::SliceFieldTake { source } => self.collect_expr_types(source, types),
            IrExprKind::Index { base, index } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(index, types);
            }
            IrExprKind::ArrayLen { array } => self.collect_expr_types(array, types),
            IrExprKind::SliceAppend { slice, item } => {
                self.collect_expr_types(slice, types);
                self.collect_expr_types(item, types);
            }
            IrExprKind::Call { args, .. } => {
                for arg in args {
                    self.collect_expr_types(&arg.expr, types);
                }
            }
            IrExprKind::IndirectCall { callee, args } => {
                self.collect_expr_types(callee, types);
                for arg in args {
                    self.collect_expr_types(&arg.expr, types);
                }
            }
            IrExprKind::ClosureValue { captures, .. } => {
                for capture in captures {
                    self.collect_expr_types(&capture.expr, types);
                }
            }
            IrExprKind::Unary { expr, .. } => self.collect_expr_types(expr, types),
            IrExprKind::Binary { left, right, .. } => {
                self.collect_expr_types(left, types);
                self.collect_expr_types(right, types);
            }
            IrExprKind::Int(_)
            | IrExprKind::String(_)
            | IrExprKind::Bool(_)
            | IrExprKind::FunctionValue { .. }
            | IrExprKind::Var(_) => {}
        }
    }

    fn typedef_for_adt(&self, ty: &Type) -> Result<String, CompileError> {
        match ty {
            Type::Option(inner) => Ok(format!(
                "typedef struct {{\n    int32_t tag;\n    {} some;\n}} {};\n",
                inner.c_name(),
                ty.c_name()
            )),
            Type::Result(ok, err) => Ok(format!(
                "typedef struct {{\n    int32_t tag;\n    {} ok;\n    {} err;\n}} {};\n",
                ok.c_name(),
                err.c_name(),
                ty.c_name()
            )),
            _ => Err(CompileError::new("internal error: expected ADT type")),
        }
    }

    fn typedef_for_struct(&self, struct_def: &IrStruct) -> String {
        let mut output = String::new();
        output.push_str("typedef struct {\n");
        for field in &struct_def.fields {
            output.push_str("    ");
            output.push_str(&field.ty.c_name());
            output.push(' ');
            output.push_str(&c_field(&field.name));
            output.push_str(";\n");
        }
        output.push_str("} ");
        output.push_str(&Type::Struct(struct_def.name.clone()).c_name());
        output.push_str(";\n");
        output
    }

    fn typedef_for_enum(&self, enum_def: &IrEnum) -> String {
        let mut output = String::from("typedef struct {\n    int32_t tag;\n");
        let payload_variants = enum_def
            .variants
            .iter()
            .filter_map(|variant| variant.payload.as_ref().map(|payload| (variant, payload)))
            .collect::<Vec<_>>();
        if !payload_variants.is_empty() {
            output.push_str("    union {\n");
            for (variant, payload) in payload_variants {
                output.push_str(&format!(
                    "        {} {};\n",
                    payload.c_name(),
                    c_field(&variant.name)
                ));
            }
            output.push_str(&format!("    }} {};\n", c_field("payload")));
        }
        output.push_str(&format!(
            "}} {};\n",
            Type::Enum(enum_def.name.clone()).c_name()
        ));
        output
    }

    fn typedef_for_array(&self, ty: &Type) -> Result<String, CompileError> {
        let Type::Array { len, element } = ty else {
            return Err(CompileError::new("internal error: expected array type"));
        };

        let mut output = String::new();
        output.push_str("typedef struct {\n");
        if *len == 0 {
            output.push_str("    char ");
            output.push_str(&c_field("empty"));
            output.push_str(";\n");
        } else {
            output.push_str("    ");
            output.push_str(&element.c_name());
            output.push(' ');
            output.push_str(&c_field("data"));
            output.push('[');
            output.push_str(&len.to_string());
            output.push_str("];\n");
        }
        output.push_str("} ");
        output.push_str(&ty.c_name());
        output.push_str(";\n");
        Ok(output)
    }

    fn typedef_for_slice(&self, ty: &Type) -> Result<String, CompileError> {
        let Type::Slice(element) = ty else {
            return Err(CompileError::new("internal error: expected slice type"));
        };

        Ok(format!(
            "typedef struct {{\n    {} *{};\n    int64_t {};\n    int64_t {};\n}} {};\n",
            element.c_name(),
            c_field("data"),
            c_field("len"),
            c_field("cap"),
            ty.c_name()
        ))
    }

    fn typedef_for_function(&self, ty: &Type) -> Result<String, CompileError> {
        let Type::Function(function) = ty else {
            return Err(CompileError::new("internal error: expected function type"));
        };
        let mut params = vec!["void *mlg_env".to_string()];
        params.extend(function.params.iter().enumerate().map(|(index, param)| {
            format!("{} mlg_arg_{index}", param.ty.c_param_type(param.mode))
        }));

        Ok(format!(
            "typedef struct {{\n    void *mlg_env;\n    void (*mlg_drop)(void *);\n    {} (*mlg_call)({});\n}} {};\n",
            function.return_type.c_name(),
            params.join(", "),
            ty.c_name()
        ))
    }

    fn emit_drop_helper(
        &self,
        ty: &Type,
        emitted: &mut Vec<Type>,
        visiting: &mut Vec<Type>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        if emitted.contains(ty) || !ty.needs_cleanup() {
            return Ok(());
        }
        if visiting.contains(ty) {
            return Err(CompileError::new(format!(
                "recursive cleanup helper involving `{}` is not supported in v0",
                ty.source_name()
            )));
        }

        visiting.push(ty.clone());
        match ty {
            Type::Option(inner) | Type::Array { element: inner, .. } | Type::Slice(inner) => {
                self.emit_drop_helper(inner, emitted, visiting, output)?;
            }
            Type::Result(ok, err) => {
                self.emit_drop_helper(ok, emitted, visiting, output)?;
                self.emit_drop_helper(err, emitted, visiting, output)?;
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                for field in &struct_def.fields {
                    self.emit_drop_helper(&field.ty, emitted, visiting, output)?;
                }
            }
            Type::Enum(name) => {
                let enum_def = self.enum_def(name)?;
                for variant in &enum_def.variants {
                    if let Some(payload) = &variant.payload {
                        self.emit_drop_helper(payload, emitted, visiting, output)?;
                    }
                }
            }
            Type::Function(_) => {}
            Type::Int | Type::Bool | Type::String | Type::Unit => {}
        }
        visiting.pop();

        output.push_str(&self.drop_helper_for_type(ty)?);
        output.push('\n');
        emitted.push(ty.clone());
        Ok(())
    }

    fn drop_helper_for_type(&self, ty: &Type) -> Result<String, CompileError> {
        let mut output = format!(
            "static void MLG_UNUSED {}({} *mlg_value) {{\n",
            drop_fn_name(ty),
            ty.c_name()
        );
        let body = self.drop_helper_body(ty)?;
        if body.is_empty() {
            push_indented_lines(&mut output, "(void)mlg_value;", 1);
        } else {
            push_indented_lines(&mut output, &body, 1);
        }
        output.push_str("}\n");
        Ok(output)
    }

    fn drop_helper_body(&self, ty: &Type) -> Result<String, CompileError> {
        match ty {
            Type::Slice(element) => {
                let mut output = String::new();
                if element.needs_cleanup() {
                    output.push_str(&format!(
                        "for (int64_t mlg_i = 0; mlg_i < mlg_value->{}; mlg_i = mlg_i + 1) {{\n",
                        c_field("len")
                    ));
                    push_indented_lines(
                        &mut output,
                        &format!(
                            "{}(&(mlg_value->{}[mlg_i]));",
                            drop_fn_name(element),
                            c_field("data")
                        ),
                        1,
                    );
                    output.push_str("}\n");
                }
                output.push_str(&format!("free(mlg_value->{});\n", c_field("data")));
                output.push_str(&format!("mlg_value->{} = NULL;\n", c_field("data")));
                output.push_str(&format!("mlg_value->{} = 0;\n", c_field("len")));
                output.push_str(&format!("mlg_value->{} = 0;", c_field("cap")));
                Ok(output)
            }
            Type::Option(inner) => {
                if !inner.needs_cleanup() {
                    return Ok(String::new());
                }
                Ok(format!(
                    "if (mlg_value->tag == 1) {{\n    {}(&(mlg_value->some));\n}}",
                    drop_fn_name(inner)
                ))
            }
            Type::Result(ok, err) => {
                let mut output = String::new();
                if ok.needs_cleanup() {
                    output.push_str(&format!(
                        "if (mlg_value->tag == 0) {{\n    {}(&(mlg_value->ok));\n}}\n",
                        drop_fn_name(ok)
                    ));
                }
                if err.needs_cleanup() {
                    if !output.is_empty() {
                        output.push_str("else ");
                    }
                    output.push_str(&format!(
                        "if (mlg_value->tag == 1) {{\n    {}(&(mlg_value->err));\n}}",
                        drop_fn_name(err)
                    ));
                }
                Ok(output)
            }
            Type::Array { len, element } => {
                if *len == 0 || !element.needs_cleanup() {
                    return Ok(String::new());
                }
                let mut output =
                    format!("for (int64_t mlg_i = 0; mlg_i < {len}; mlg_i = mlg_i + 1) {{\n");
                push_indented_lines(
                    &mut output,
                    &format!(
                        "{}(&(mlg_value->{}[mlg_i]));",
                        drop_fn_name(element),
                        c_field("data")
                    ),
                    1,
                );
                output.push('}');
                Ok(output)
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                let mut output = String::new();
                for field in &struct_def.fields {
                    if !field.ty.needs_cleanup() {
                        continue;
                    }
                    output.push_str(&format!(
                        "{}(&(mlg_value->{}));\n",
                        drop_fn_name(&field.ty),
                        c_field(&field.name)
                    ));
                }
                if output.ends_with('\n') {
                    output.pop();
                }
                Ok(output)
            }
            Type::Enum(name) => {
                let enum_def = self.enum_def(name)?;
                let mut output = String::new();
                for (tag, variant) in enum_def.variants.iter().enumerate() {
                    let Some(payload) = variant
                        .payload
                        .as_ref()
                        .filter(|payload| payload.needs_cleanup())
                    else {
                        continue;
                    };
                    if !output.is_empty() {
                        output.push_str("else ");
                    }
                    output.push_str(&format!(
                        "if (mlg_value->tag == {tag}) {{\n    {}(&(mlg_value->{}.{}));\n}}",
                        drop_fn_name(payload),
                        c_field("payload"),
                        c_field(&variant.name)
                    ));
                    output.push('\n');
                }
                if output.ends_with('\n') {
                    output.pop();
                }
                Ok(output)
            }
            Type::Function(_) => Ok(
                "if (mlg_value->mlg_drop != NULL) {\n    mlg_value->mlg_drop(mlg_value->mlg_env);\n}\nmlg_value->mlg_env = NULL;\nmlg_value->mlg_drop = NULL;\nmlg_value->mlg_call = NULL;"
                    .to_string(),
            ),
            Type::Int | Type::Bool | Type::String | Type::Unit => Err(CompileError::new(format!(
                "IR invariant violation: drop helper requested for non-cleanup type `{}`",
                ty.source_name()
            ))),
        }
    }
}

fn collect_type(ty: &Type, types: &mut Vec<Type>) {
    match ty {
        Type::Option(inner) => {
            collect_type(inner, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Result(ok, err) => {
            collect_type(ok, types);
            collect_type(err, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Struct(_) | Type::Enum(_) => {
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Array { element, .. } => {
            collect_type(element, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Slice(element) => {
            collect_type(element, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Function(function) => {
            for param in &function.params {
                collect_type(&param.ty, types);
            }
            collect_type(&function.return_type, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Int | Type::Bool | Type::String | Type::Unit => {}
    }
}
