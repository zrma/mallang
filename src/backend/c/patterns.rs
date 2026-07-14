use std::collections::HashMap;

use crate::{ir::IrMatchPattern, semantic::Type};

use super::{names::c_field, CGenerator, CompileError};

pub(super) struct CPatternPlan {
    pub(super) condition: String,
    pub(super) env: HashMap<String, String>,
}

impl<'a> CGenerator<'a> {
    pub(super) fn plan_user_enum_pattern(
        &self,
        pattern: &IrMatchPattern,
        expected: &Type,
        value: &str,
        env: &HashMap<String, String>,
    ) -> Result<CPatternPlan, CompileError> {
        let mut conditions = Vec::new();
        let mut arm_env = env.clone();
        self.plan_pattern(pattern, expected, value, &mut conditions, &mut arm_env)?;
        Ok(CPatternPlan {
            condition: if conditions.is_empty() {
                "true".to_string()
            } else {
                conditions.join(" && ")
            },
            env: arm_env,
        })
    }

    fn plan_pattern(
        &self,
        pattern: &IrMatchPattern,
        expected: &Type,
        value: &str,
        conditions: &mut Vec<String>,
        env: &mut HashMap<String, String>,
    ) -> Result<(), CompileError> {
        match pattern {
            IrMatchPattern::Wildcard(ty) => {
                self.expect_pattern_type(ty, expected)?;
            }
            IrMatchPattern::Binding { name, ty } => {
                self.expect_pattern_type(ty, expected)?;
                env.insert(name.clone(), value.to_string());
            }
            IrMatchPattern::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                let Type::Enum(expected_name) = expected else {
                    return Err(CompileError::new(
                        "IR invariant violation: user enum pattern on non-enum value",
                    ));
                };
                if enum_name != expected_name {
                    return Err(CompileError::new(
                        "IR invariant violation: user enum pattern type mismatch",
                    ));
                }
                let enum_def = self.enum_def(enum_name)?;
                let (tag, variant_def) = enum_def
                    .variants
                    .iter()
                    .enumerate()
                    .find(|(_, candidate)| candidate.name == *variant)
                    .ok_or_else(|| {
                        CompileError::new(format!(
                            "IR invariant violation: unknown enum variant `{enum_name}.{variant}`"
                        ))
                    })?;
                conditions.push(format!("({value}).tag == {tag}"));
                match (&variant_def.payload, payload.as_deref()) {
                    (None, None) => {}
                    (Some(payload_ty), Some(payload_pattern)) => {
                        let payload_value =
                            format!("({value}).{}.{}", c_field("payload"), c_field(variant));
                        self.plan_pattern(
                            payload_pattern,
                            payload_ty,
                            &payload_value,
                            conditions,
                            env,
                        )?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            "IR invariant violation: user enum pattern payload mismatch",
                        ));
                    }
                }
            }
            IrMatchPattern::NestedBuiltin { variant, payload } => {
                let (tag, payload_ty, field) = match (expected, variant.as_str()) {
                    (Type::Option(_), "None") => (0, None, None),
                    (Type::Option(inner), "Some") => (1, Some(inner.as_ref()), Some("some")),
                    (Type::Result(ok, _), "Ok") => (0, Some(ok.as_ref()), Some("ok")),
                    (Type::Result(_, err), "Err") => (1, Some(err.as_ref()), Some("err")),
                    _ => {
                        return Err(CompileError::new(
                            "IR invariant violation: invalid nested built-in pattern",
                        ));
                    }
                };
                conditions.push(format!("({value}).tag == {tag}"));
                match (payload_ty, field, payload.as_deref()) {
                    (None, None, None) => {}
                    (Some(payload_ty), Some(field), Some(payload_pattern)) => {
                        let payload_value = format!("({value}).{field}");
                        self.plan_pattern(
                            payload_pattern,
                            payload_ty,
                            &payload_value,
                            conditions,
                            env,
                        )?;
                    }
                    _ => {
                        return Err(CompileError::new(
                            "IR invariant violation: nested built-in pattern payload mismatch",
                        ));
                    }
                }
            }
            IrMatchPattern::Some(_)
            | IrMatchPattern::None
            | IrMatchPattern::Ok(_)
            | IrMatchPattern::Err(_) => {
                return Err(CompileError::new(
                    "IR invariant violation: legacy flat pattern in user enum match",
                ));
            }
        }
        Ok(())
    }

    fn expect_pattern_type(&self, actual: &Type, expected: &Type) -> Result<(), CompileError> {
        if actual == expected {
            Ok(())
        } else {
            Err(CompileError::new(
                "IR invariant violation: match binding type mismatch",
            ))
        }
    }
}
