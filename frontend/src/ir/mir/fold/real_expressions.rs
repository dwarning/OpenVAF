/*
 * ******************************************************************************************
 * Copyright (c) 2020 Pascal Kuthe. This file is part of the frontend project.
 * It is subject to the license terms in the LICENSE file found in the top-level directory
 *  of this distribution and at  https://gitlab.com/DSPOM/OpenVAF/blob/master/LICENSE.
 *  No part of frontend, including this file, may be copied, modified, propagated, or
 *  distributed except according to the terms contained in the LICENSE file.
 * *****************************************************************************************
 */

use crate::ir::hir::DisciplineAccess;
use crate::ir::mir::RealBinaryOperator;
use crate::ir::{
    BranchId, BuiltInFunctionCall1p, BuiltInFunctionCall2p, IntegerExpressionId, Node, NoiseSource,
    ParameterId, RealExpressionId, StringExpressionId, VariableId,
};
use crate::mir::{Mir, RealExpression};
use crate::sourcemap::Span;
use crate::StringLiteral;

pub fn walk_real_expression<V: RealExprFold>(fold: &mut V, expr: RealExpressionId) -> V::T {
    let mir = fold.mir();
    match mir[expr].contents {
        RealExpression::Literal(val) => fold.fold_literal(val),
        RealExpression::BinaryOperator(lhs, op, rhs) => fold.fold_binary_operator(lhs, op, rhs),
        RealExpression::Negate(op, expr) => fold.fold_negate(op, expr),
        RealExpression::Condition(cond, true_expr, false_expr) => {
            fold.fold_condition(cond, true_expr, false_expr)
        }
        RealExpression::VariableReference(var) => fold.fold_variable_reference(var),
        RealExpression::ParameterReference(param) => fold.fold_parameter_reference(param),
        RealExpression::BranchAccess(discipline_access, branch, derivative_order) => {
            fold.fold_branch_access(discipline_access, branch, derivative_order)
        }
        RealExpression::Noise(noise_src, name) => fold.fold_noise(noise_src, name),
        RealExpression::BuiltInFunctionCall1p(call, arg) => {
            fold.fold_builtin_function_call_1p(call, arg)
        }
        RealExpression::BuiltInFunctionCall2p(call, arg1, arg2) => {
            fold.fold_builtin_function_call_2p(call, arg1, arg2)
        }
        RealExpression::IntegerConversion(integer_expr) => {
            fold.fold_integer_conversion(integer_expr)
        }
        RealExpression::Temperature => fold.fold_temperature(),
        RealExpression::SimParam(name, default) => fold.fold_sim_param(name, default),
    }
}

pub trait RealExprFold: Sized {
    type T;
    fn mir(&self) -> &Mir;

    #[inline]
    fn fold_real_expr(&mut self, expr: RealExpressionId) -> Self::T {
        walk_real_expression(self, expr)
    }

    fn fold_literal(&mut self, val: f64) -> Self::T;
    fn fold_binary_operator(
        &mut self,
        lhs: RealExpressionId,
        op: Node<RealBinaryOperator>,
        rhs: RealExpressionId,
    ) -> Self::T;
    fn fold_negate(&mut self, op: Span, arg: RealExpressionId) -> Self::T;
    fn fold_condition(
        &mut self,
        cond: IntegerExpressionId,
        true_expr: RealExpressionId,
        false_expr: RealExpressionId,
    ) -> Self::T;
    fn fold_variable_reference(&mut self, var: VariableId) -> Self::T;
    fn fold_parameter_reference(&mut self, param: ParameterId) -> Self::T;
    fn fold_branch_access(
        &mut self,
        discipline_accesss: DisciplineAccess,
        branch: BranchId,
        time_derivative_order: u8,
    ) -> Self::T;
    fn fold_noise(
        &mut self,
        noise_src: NoiseSource<RealExpressionId, ()>,
        name: Option<StringLiteral>,
    ) -> Self::T;
    fn fold_builtin_function_call_1p(
        &mut self,
        call: BuiltInFunctionCall1p,
        arg: RealExpressionId,
    ) -> Self::T;
    fn fold_builtin_function_call_2p(
        &mut self,
        call: BuiltInFunctionCall2p,
        arg1: RealExpressionId,
        arg2: RealExpressionId,
    ) -> Self::T;
    fn fold_temperature(&mut self) -> Self::T;
    fn fold_sim_param(
        &mut self,
        name: StringExpressionId,
        default: Option<RealExpressionId>,
    ) -> Self::T;
    fn fold_integer_conversion(&mut self, expr: IntegerExpressionId) -> Self::T;
}

#[inline]
pub fn walk_real_binary_operator<V: RealBinaryOperatorFold>(
    fold: &mut V,
    lhs: RealExpressionId,
    op: RealBinaryOperator,
    rhs: RealExpressionId,
) -> V::T {
    match op {
        RealBinaryOperator::Sum => fold.fold_sum(lhs, rhs),
        RealBinaryOperator::Subtract => fold.fold_diff(lhs, rhs),
        RealBinaryOperator::Multiply => fold.fold_mul(lhs, rhs),
        RealBinaryOperator::Divide => fold.fold_quotient(lhs, rhs),
        RealBinaryOperator::Exponent => fold.fold_pow(lhs, rhs),
        RealBinaryOperator::Modulus => fold.fold_mod(lhs, rhs),
    }
}

pub trait RealBinaryOperatorFold: Sized {
    type T;

    fn fold_real_binary_op(
        &mut self,
        lhs: RealExpressionId,
        op: RealBinaryOperator,
        rhs: RealExpressionId,
    ) -> Self::T {
        walk_real_binary_operator(self, lhs, op, rhs)
    }

    fn fold_sum(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
    fn fold_diff(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
    fn fold_mul(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
    fn fold_quotient(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
    fn fold_pow(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
    fn fold_mod(&mut self, lhs: RealExpressionId, rhs: RealExpressionId) -> Self::T;
}

#[inline]
pub fn walk_real_builtin_function_call_1p<V: RealBuiltInFunctionCall1pFold>(
    fold: &mut V,
    call: BuiltInFunctionCall1p,
    arg: RealExpressionId,
) -> V::T {
    match call {
        BuiltInFunctionCall1p::Sqrt => fold.fold_sqrt(arg),
        BuiltInFunctionCall1p::Exp(_) => fold.fold_exp(arg),
        BuiltInFunctionCall1p::Ln => fold.fold_ln(arg),
        BuiltInFunctionCall1p::Log => fold.fold_log(arg),
        BuiltInFunctionCall1p::Abs => fold.fold_abs(arg),
        BuiltInFunctionCall1p::Floor => fold.fold_floor(arg),
        BuiltInFunctionCall1p::Ceil => fold.fold_ceil(arg),
        BuiltInFunctionCall1p::Sin => fold.fold_sin(arg),
        BuiltInFunctionCall1p::Cos => fold.fold_cos(arg),
        BuiltInFunctionCall1p::Tan => fold.fold_tan(arg),
        BuiltInFunctionCall1p::ArcSin => fold.fold_arcsin(arg),
        BuiltInFunctionCall1p::ArcCos => fold.fold_arccos(arg),
        BuiltInFunctionCall1p::ArcTan => fold.fold_arctan(arg),
        BuiltInFunctionCall1p::SinH => fold.fold_sinh(arg),
        BuiltInFunctionCall1p::CosH => fold.fold_cosh(arg),
        BuiltInFunctionCall1p::TanH => fold.fold_tanh(arg),
        BuiltInFunctionCall1p::ArcSinH => fold.fold_arcsinh(arg),
        BuiltInFunctionCall1p::ArcCosH => fold.fold_arccosh(arg),
        BuiltInFunctionCall1p::ArcTanH => fold.fold_arctanh(arg),
    }
}

pub trait RealBuiltInFunctionCall1pFold: Sized {
    type T;

    #[inline]
    fn fold_real_builtin_function_call_1p(
        &mut self,
        call: BuiltInFunctionCall1p,
        arg: RealExpressionId,
    ) -> Self::T {
        walk_real_builtin_function_call_1p(self, call, arg)
    }

    fn fold_sqrt(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_exp(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_ln(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_log(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_abs(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_floor(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_ceil(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_sin(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_cos(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_tan(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arcsin(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arccos(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arctan(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_sinh(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_cosh(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_tanh(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arcsinh(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arccosh(&mut self, arg: RealExpressionId) -> Self::T;
    fn fold_arctanh(&mut self, arg: RealExpressionId) -> Self::T;
}

#[inline]
pub fn walk_real_builtin_function_call_2p<V: RealBuiltInFunctionCall2pFold>(
    fold: &mut V,
    call: BuiltInFunctionCall2p,
    arg1: RealExpressionId,
    arg2: RealExpressionId,
) -> V::T {
    match call {
        BuiltInFunctionCall2p::Pow => fold.fold_pow(arg1, arg2),
        BuiltInFunctionCall2p::Hypot => fold.fold_hypot(arg1, arg2),
        BuiltInFunctionCall2p::ArcTan2 => fold.fold_arctan2(arg1, arg2),
        BuiltInFunctionCall2p::Max => fold.fold_max(arg1, arg2),
        BuiltInFunctionCall2p::Min => fold.fold_min(arg1, arg2),
    }
}

pub trait RealBuiltInFunctionCall2pFold: Sized {
    type T;

    fn fold_real_builtin_function_call_2p(
        &mut self,
        call: BuiltInFunctionCall2p,
        arg1: RealExpressionId,
        arg2: RealExpressionId,
    ) -> Self::T {
        walk_real_builtin_function_call_2p(self, call, arg1, arg2)
    }

    fn fold_pow(&mut self, arg1: RealExpressionId, arg2: RealExpressionId) -> Self::T;
    fn fold_hypot(&mut self, arg1: RealExpressionId, arg2: RealExpressionId) -> Self::T;
    fn fold_arctan2(&mut self, arg1: RealExpressionId, arg2: RealExpressionId) -> Self::T;
    fn fold_max(&mut self, arg1: RealExpressionId, arg2: RealExpressionId) -> Self::T;
    fn fold_min(&mut self, arg1: RealExpressionId, arg2: RealExpressionId) -> Self::T;
}