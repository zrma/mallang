use std::collections::HashMap;

use crate::{
    ir::{IrEnumStorage, IrMatchPattern},
    semantic::Type,
    token::Span,
};

use super::{
    names::{c_field, variant_payload_member, TypeCName},
    utils::{pattern_binding_env_key, pattern_binding_temp_name},
    CGenerator, CompileError,
};

pub(super) struct CPatternPlan {
    pub(super) condition: String,
    pub(super) setup: Vec<String>,
    pub(super) env: HashMap<String, String>,
}

struct PatternPlanContext<'a> {
    binding_span: Span,
    conditions: &'a mut Vec<String>,
    setup: &'a mut Vec<String>,
    env: &'a mut HashMap<String, String>,
}

impl<'a> CGenerator<'a> {
    pub(super) fn plan_adt_pattern(
        &self,
        pattern: &IrMatchPattern,
        expected: &Type,
        value: &str,
        binding_span: Span,
        env: &HashMap<String, String>,
    ) -> Result<CPatternPlan, CompileError> {
        let mut conditions = Vec::new();
        let mut setup = Vec::new();
        let mut arm_env = env.clone();
        let mut context = PatternPlanContext {
            binding_span,
            conditions: &mut conditions,
            setup: &mut setup,
            env: &mut arm_env,
        };
        self.plan_pattern(pattern, expected, value, &mut context)?;
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
        context: &mut PatternPlanContext<'_>,
    ) -> Result<(), CompileError> {
        match pattern {
            IrMatchPattern::Wildcard(ty) => {
                self.expect_pattern_type(ty, expected)?;
            }
            IrMatchPattern::Binding { name, ty } => {
                self.expect_pattern_type(ty, expected)?;
                let binding = pattern_binding_temp_name(name, context.binding_span);
                context
                    .setup
                    .push(format!("{} {binding} = {value};", ty.c_name()));
                context.setup.push(format!("(void)&{binding};"));
                context.env.insert(name.clone(), binding.clone());
                context
                    .env
                    .insert(pattern_binding_env_key(name, context.binding_span), binding);
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
                        context.conditions.push(format!(
                            "({node} != NULL || (mallang_runtime_error(\"invalid recursive enum handle\"), false))"
                        ));
                        (format!("{node}->tag"), Some(node))
                    }
                };
                context.conditions.push(format!("{tag_value} == {tag}"));
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
                    self.plan_pattern(payload_pattern, payload_ty, &payload_value, context)?;
                }
                if let Some(node) = node {
                    context.setup.push(format!("mallang_dealloc({node});"));
                    context.setup.push(format!("{node} = NULL;"));
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
