use std::collections::HashMap;

use crate::{ir::IrMatchPattern, semantic::Type};

use super::{names::c_field, CGenerator, CompileError};

pub(super) struct CPatternPlan {
    pub(super) condition: String,
    pub(super) env: HashMap<String, String>,
}

impl<'a> CGenerator<'a> {
    pub(super) fn plan_adt_pattern(
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
            IrMatchPattern::Variant {
                ty,
                variant,
                payload,
            } => {
                self.expect_pattern_type(ty, expected)?;
                let (tag, payload_ty) = self.adt_variant(expected, variant)?;
                conditions.push(format!("({value}).tag == {tag}"));
                match (payload_ty, payload.as_deref()) {
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
                            "IR invariant violation: ADT pattern payload mismatch",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn adt_variant<'b>(
        &'b self,
        ty: &'b Type,
        variant: &str,
    ) -> Result<(usize, Option<&'b Type>), CompileError> {
        match ty {
            Type::Option(inner) => match variant {
                "None" => Ok((0, None)),
                "Some" => Ok((1, Some(inner.as_ref()))),
                _ => Err(CompileError::new(format!(
                    "IR invariant violation: unknown ADT variant `Option.{variant}`"
                ))),
            },
            Type::Result(ok, err) => match variant {
                "Ok" => Ok((0, Some(ok.as_ref()))),
                "Err" => Ok((1, Some(err.as_ref()))),
                _ => Err(CompileError::new(format!(
                    "IR invariant violation: unknown ADT variant `Result.{variant}`"
                ))),
            },
            Type::Enum(name) => self
                .enum_def(name)?
                .variants
                .iter()
                .enumerate()
                .find(|(_, candidate)| candidate.name == variant)
                .map(|(tag, variant)| (tag, variant.payload.as_ref()))
                .ok_or_else(|| {
                    CompileError::new(format!(
                        "IR invariant violation: unknown enum variant `{name}.{variant}`"
                    ))
                }),
            _ => Err(CompileError::new(
                "IR invariant violation: variant requested for non-ADT type",
            )),
        }
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
