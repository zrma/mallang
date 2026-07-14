use std::collections::HashMap;

use crate::{
    ir::{IrEnumStorage, IrMatchPattern},
    semantic::Type,
};

use super::{
    names::{c_field, c_ident, variant_payload_member, TypeCName},
    CGenerator, CompileError,
};

pub(super) struct CPatternPlan {
    pub(super) condition: String,
    pub(super) setup: Vec<String>,
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
        let mut setup = Vec::new();
        let mut arm_env = env.clone();
        self.plan_pattern(
            pattern,
            expected,
            value,
            &mut conditions,
            &mut setup,
            &mut arm_env,
        )?;
        Ok(CPatternPlan {
            condition: if conditions.is_empty() {
                "true".to_string()
            } else {
                conditions.join(" && ")
            },
            setup,
            env: arm_env,
        })
    }

    fn plan_pattern(
        &self,
        pattern: &IrMatchPattern,
        expected: &Type,
        value: &str,
        conditions: &mut Vec<String>,
        setup: &mut Vec<String>,
        env: &mut HashMap<String, String>,
    ) -> Result<(), CompileError> {
        match pattern {
            IrMatchPattern::Wildcard(ty) => {
                self.expect_pattern_type(ty, expected)?;
            }
            IrMatchPattern::Binding { name, ty } => {
                self.expect_pattern_type(ty, expected)?;
                let binding = c_ident(name);
                setup.push(format!("{} {binding} = {value};", ty.c_name()));
                setup.push(format!("(void)&{binding};"));
                env.insert(name.clone(), binding);
            }
            IrMatchPattern::Variant {
                ty,
                variant,
                storage,
                payloads,
            } => {
                self.expect_pattern_type(ty, expected)?;
                let (tag, payload_types) = self.adt_variant(expected, variant)?;
                let (tag_value, node) = match storage {
                    IrEnumStorage::Inline => (format!("({value}).tag"), None),
                    IrEnumStorage::Owned => {
                        let node = format!("({value}).{}", c_field("node"));
                        conditions.push(format!(
                            "({node} != NULL || (mallang_runtime_error(\"invalid recursive enum handle\"), false))"
                        ));
                        (format!("{node}->tag"), Some(node))
                    }
                };
                conditions.push(format!("{tag_value} == {tag}"));
                if payload_types.len() != payloads.len() {
                    return Err(CompileError::new(
                        "IR invariant violation: ADT pattern payload mismatch",
                    ));
                }
                for (index, (payload_ty, payload_pattern)) in
                    payload_types.into_iter().zip(payloads).enumerate()
                {
                    let member = variant_payload_member(variant, payloads.len(), index);
                    let payload_value = match &node {
                        Some(node) => format!("{node}->{member}"),
                        None => format!("({value}).{member}"),
                    };
                    self.plan_pattern(
                        payload_pattern,
                        payload_ty,
                        &payload_value,
                        conditions,
                        setup,
                        env,
                    )?;
                }
                if let Some(node) = node {
                    setup.push(format!("mallang_dealloc({node});"));
                    setup.push(format!("{node} = NULL;"));
                }
            }
        }
        Ok(())
    }

    pub(super) fn adt_variant<'b>(
        &'b self,
        ty: &'b Type,
        variant: &str,
    ) -> Result<(usize, Vec<&'b Type>), CompileError> {
        match ty {
            Type::Option(inner) => match variant {
                "None" => Ok((0, Vec::new())),
                "Some" => Ok((1, vec![inner.as_ref()])),
                _ => Err(CompileError::new(format!(
                    "IR invariant violation: unknown ADT variant `Option.{variant}`"
                ))),
            },
            Type::Result(ok, err) => match variant {
                "Ok" => Ok((0, vec![ok.as_ref()])),
                "Err" => Ok((1, vec![err.as_ref()])),
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
                .map(|(tag, variant)| (tag, variant.payloads.iter().collect()))
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
