use ahash::AHashMap;
use bitset::BitSet;
use hir_def::db::HirDefDB;
use hir_def::ParamSysFun;
use hir_lower::{HirInterner, ParamKind, PlaceKind, REACTIVE_DIM, RESISTIVE_DIM};
use hir_ty::inference::BranchWrite;
use indexmap::map::Entry;
use mir::builder::InstBuilder;
use mir::cursor::{Cursor, FuncCursor};
use mir::{ControlFlowGraph, DerivativeInfo, Function, Inst, Value, FALSE, F_ZERO, TRUE};
use stdx::{impl_debug_display, impl_idx_from};
use typed_indexmap::TiMap;

use crate::compilation_db::CompilationDB;
use crate::{strip_optbarrier, SimUnkown};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ResidualId(u32);
impl_idx_from!(ResidualId(u32));
impl_debug_display! {match ResidualId{ResidualId(id) => "res{id}";}}

fn get_contrib(
    cursor: &mut FuncCursor,
    intern: &HirInterner,
    dst: BranchWrite,
    react: bool,
    voltage_src: bool,
) -> Value {
    let dim = if react { REACTIVE_DIM } else { RESISTIVE_DIM };
    let contrib = PlaceKind::Contribute { dst, dim, voltage_src };

    intern
        .outputs
        .get(&contrib)
        .and_then(|val| val.expand())
        .map(|val| strip_optbarrier(cursor.func, val))
        .unwrap_or(F_ZERO)
}

fn src_residual(
    cursor: &mut FuncCursor,
    intern: &mut HirInterner,
    dst: BranchWrite,
    voltage_src: bool,
    db: &CompilationDB,
) -> [Value; 2] {
    let resist_val = get_contrib(cursor, intern, dst, false, voltage_src);
    let react_val = get_contrib(cursor, intern, dst, true, voltage_src);

    let unkown = if voltage_src {
        let (hi, lo) = dst.nodes(db);
        ParamKind::Voltage { hi, lo }
    } else {
        ParamKind::Current(dst.into())
    };
    let unkown = intern.ensure_param(cursor.func, unkown);

    let resit_residual = if resist_val == F_ZERO {
        if react_val == F_ZERO {
            unkown
        } else {
            cursor.ins().fneg(unkown)
        }
    } else {
        cursor.ins().fsub(resist_val, unkown)
    };

    [resit_residual, react_val]
}

#[derive(Default, Debug)]
pub struct Residual {
    pub resistive: TiMap<ResidualId, SimUnkown, Value>,
    pub reactive: TiMap<ResidualId, SimUnkown, Value>,
}

impl Residual {
    pub fn populate(
        &mut self,
        db: &CompilationDB,
        cursor: &mut FuncCursor,
        intern: &mut HirInterner,
        cfg: &mut ControlFlowGraph,
        op_dependent: &BitSet<Inst>,
    ) {
        for i in 0..intern.outputs.len() {
            let (kind, val) = intern.outputs.get_index(i).unwrap();

            let dst = match *kind {
                PlaceKind::IsVoltageSrc(dst) => dst,
                PlaceKind::ImplicitResidual {
                    equation,
                    dim: dim @ (RESISTIVE_DIM | REACTIVE_DIM),
                } => {
                    self.add_entry(
                        cursor,
                        SimUnkown::Implicit(equation),
                        val.unwrap_unchecked(),
                        false,
                        dim == REACTIVE_DIM,
                    );
                    continue;
                }

                _ => continue,
            };

            // let (hi, lo) = dst.nodes(db);
            // let hi = db.node_data(hi).name.to_string();
            // let lo = lo.map_or_else(|| "gnd".to_string(), |hi| db.node_data(hi).name.to_string());
            // println!("{hi} {lo}");

            let current = dst.into();
            let is_voltage_src = strip_optbarrier(cursor.func, val.unwrap_unchecked());

            let residual = match is_voltage_src {
                FALSE => {
                    let requires_unkown =
                        intern.is_param_live(cursor.func, &ParamKind::Current(current));
                    if requires_unkown {
                        src_residual(cursor, intern, dst, false, db)
                    } else {
                        // no extra unkowns required

                        let resist_val = get_contrib(cursor, intern, dst, false, false);
                        if resist_val != F_ZERO {
                            self.add_kirchoff_laws(cursor, resist_val, dst, false, db);
                        }

                        let react_val = get_contrib(cursor, intern, dst, true, false);
                        if react_val != F_ZERO {
                            self.add_kirchoff_laws(cursor, react_val, dst, true, db);
                        }
                        continue;
                    }
                }

                TRUE => {
                    // voltage src

                    let requires_unkown =
                        intern.is_param_live(cursor.func, &ParamKind::Current(current));

                    let static_collapse = !requires_unkown
                        && get_contrib(cursor, intern, dst, false, true) == F_ZERO
                        && get_contrib(cursor, intern, dst, true, true) == F_ZERO;

                    if static_collapse {
                        // just node collapsing
                        continue;
                    }

                    src_residual(cursor, intern, dst, true, db)
                }

                // switch branch
                _ => {
                    let requires_unkown =
                        intern.is_param_live(cursor.func, &ParamKind::Current(current));
                    let op_dependent = cursor
                        .func
                        .dfg
                        .value_def(is_voltage_src)
                        .inst()
                        .map_or(false, |inst| op_dependent.contains(inst));

                    let resist_voltage = get_contrib(cursor, intern, dst, false, true);
                    let react_voltage = get_contrib(cursor, intern, dst, true, true);
                    let just_current_src = !requires_unkown
                        && !op_dependent
                        && resist_voltage == F_ZERO
                        && react_voltage == F_ZERO;

                    let resist_current = get_contrib(cursor, intern, dst, false, false);
                    let react_current = get_contrib(cursor, intern, dst, true, false);

                    if just_current_src {
                        // no extra unkowns required so just node collapsing + current source is
                        // enough
                        if resist_current != F_ZERO {
                            self.add_kirchoff_laws(cursor, resist_current, dst, false, db);
                        }

                        if react_current != F_ZERO {
                            self.add_kirchoff_laws(cursor, react_current, dst, true, db);
                        }
                        continue;
                    }

                    // let (hi, lo) = dst.nodes(db);
                    // unreachable!(
                    //     "hmm {}, {} = {resist_voltage} + j {react_voltage} {:?} {requires_unkown} {is_voltage_src}",
                    //     db.node_data(hi).name,
                    //     lo.map_or("gnd".to_owned(), |lo| db.node_data(lo).name.to_string()),
                    //     cursor.func.dfg.value_def(resist_current)
                    // );

                    let start_bb = cursor.current_block().unwrap();
                    let voltage_src_bb = cursor.layout_mut().append_new_block();
                    let current_src_bb = cursor.layout_mut().append_new_block();
                    let next_block = cursor.layout_mut().append_new_block();
                    cfg.ensure_bb(next_block);
                    cfg.add_edge(start_bb, voltage_src_bb);
                    cfg.add_edge(start_bb, current_src_bb);
                    cfg.add_edge(voltage_src_bb, next_block);
                    cfg.add_edge(current_src_bb, next_block);

                    cursor.ins().br(is_voltage_src, voltage_src_bb, current_src_bb);
                    cursor.goto_bottom(voltage_src_bb);
                    let voltage_residual = src_residual(cursor, intern, dst, true, db);
                    cursor.ins().jump(next_block);

                    cursor.goto_bottom(current_src_bb);
                    let introduce_unkown =
                        requires_unkown || resist_voltage != F_ZERO || react_voltage != F_ZERO;

                    let current_residual = if introduce_unkown {
                        src_residual(cursor, intern, dst, false, db)
                    } else {
                        let val = if op_dependent {
                            intern.ensure_param(cursor.func, ParamKind::Current(dst.into()))
                        } else {
                            // TODO collapse
                            // let func_ref = intern.;
                            //     cursor.ins().call(func_ref, &[]);
                            F_ZERO
                        };
                        [val, F_ZERO]
                    };
                    cursor.ins().jump(next_block);

                    cursor.goto_bottom(next_block);
                    let residual_resist = cursor.ins().phi(&[
                        (current_src_bb, current_residual[0]),
                        (voltage_src_bb, voltage_residual[0]),
                    ]);
                    let residual_react = cursor.ins().phi(&[
                        (current_src_bb, current_residual[1]),
                        (voltage_src_bb, voltage_residual[1]),
                    ]);

                    if !introduce_unkown {
                        if resist_current != F_ZERO {
                            let residual_resist = cursor
                                .ins()
                                .phi(&[(current_src_bb, resist_current), (voltage_src_bb, F_ZERO)]);
                            self.add_kirchoff_laws(cursor, residual_resist, dst, false, db);
                        }

                        if react_current != F_ZERO {
                            let residual_react = cursor
                                .ins()
                                .phi(&[(current_src_bb, react_current), (voltage_src_bb, F_ZERO)]);
                            self.add_kirchoff_laws(cursor, residual_react, dst, false, db);
                        }
                    }

                    [residual_resist, residual_react]
                }
            };

            self.add_entry(cursor, SimUnkown::Current(current), residual[0], false, false);
            self.add_entry(cursor, SimUnkown::Current(current), residual[1], false, true);

            let current = intern.ensure_param(cursor.func, ParamKind::Current(current));
            self.add_kirchoff_laws(cursor, current, dst, false, db);
        }

        let mfactor =
            intern.ensure_param(cursor.func, ParamKind::ParamSysFun(ParamSysFun::mfactor));
        for val in self.resistive.raw.values_mut().chain(self.reactive.raw.values_mut()) {
            *val = cursor.ins().fmul(*val, mfactor)
        }
    }

    pub fn jacobian_derivatives(
        &self,
        func: &Function,
        intern: &HirInterner,
        unkowns: &DerivativeInfo,
    ) -> Vec<(Value, mir::Unkown)> {
        let params: Vec<_> = intern
            .live_params(&func.dfg)
            .filter_map(move |(_, kind, param)| {
                if matches!(
                    kind,
                    ParamKind::Voltage { .. }
                        | ParamKind::Current(_)
                        | ParamKind::ImplicitUnkown(_)
                ) {
                    Some(unkowns.unkowns.unwrap_index(&param))
                } else {
                    None
                }
            })
            .collect();

        let num_unkowns = params.len() * (self.resistive.len() + self.reactive.len());
        let mut res = Vec::with_capacity(num_unkowns);
        for dim in [&self.resistive, &self.reactive] {
            for residual in dim.raw.values() {
                res.extend(params.iter().map(|unkown| (*residual, *unkown)))
            }
        }

        res
    }

    fn add_kirchoff_laws(
        &mut self,
        cursor: &mut FuncCursor,
        current: Value,
        dst: BranchWrite,
        reactive: bool,
        db: &CompilationDB,
    ) {
        let (hi, lo) = dst.nodes(db);
        self.add_entry(cursor, SimUnkown::KirchoffLaw(hi), current, false, reactive);
        if let Some(lo) = lo {
            self.add_entry(cursor, SimUnkown::KirchoffLaw(lo), current, true, reactive);
        }
    }

    pub(crate) fn insert_opt_barries(
        &mut self,
        func: &mut FuncCursor,
        output_values: &mut BitSet<Value>,
    ) {
        fn insert_opt_barries(
            entries: &mut TiMap<ResidualId, SimUnkown, Value>,
            func: &mut FuncCursor,
            output_values: &mut BitSet<Value>,
        ) {
            for val in entries.raw.values_mut() {
                *val = func.ins().optbarrier(*val);
            }

            output_values.ensure(func.func.dfg.num_values() + 1);

            for val in entries.raw.values() {
                output_values.insert(*val);
            }
        }

        insert_opt_barries(&mut self.resistive, func, output_values);
        insert_opt_barries(&mut self.reactive, func, output_values);
    }

    pub(crate) fn strip_opt_barries(
        &mut self,
        func: &mut Function,
        output_values: &mut BitSet<Value>,
    ) {
        fn strip_opt_barries(
            entries: &mut TiMap<ResidualId, SimUnkown, Value>,
            func: &mut Function,
            output_values: &mut BitSet<Value>,
        ) {
            for val in entries.raw.values_mut() {
                let inst = func.dfg.value_def(*val).unwrap_inst();
                output_values.remove(*val);
                *val = func.dfg.instr_args(inst)[0];
                output_values.insert(*val);
                func.layout.remove_inst(inst);
            }
        }

        strip_opt_barries(&mut self.resistive, func, output_values);
        strip_opt_barries(&mut self.reactive, func, output_values);
    }

    pub(crate) fn add_entry(
        &mut self,
        func: &mut FuncCursor,
        node: SimUnkown,
        mut val: Value,
        neg: bool,
        reactive: bool,
    ) {
        let dst = if reactive { &mut self.reactive } else { &mut self.resistive };
        // no entrys for gnd nodes

        match dst.raw.entry(node) {
            Entry::Occupied(dst) => {
                let dst = dst.into_mut();
                *dst = if neg { func.ins().fsub(*dst, val) } else { func.ins().fadd(*dst, val) }
            }
            Entry::Vacant(dst) => {
                if neg {
                    val = func.ins().fneg(val);
                }
                dst.insert(val);
            }
        }
    }

    pub(crate) fn sparsify(&mut self) {
        self.resistive.raw.retain(|_, val| *val != F_ZERO);
        self.reactive.raw.retain(|_, val| *val != F_ZERO);
    }

    pub fn resistive_entries(&self, db: &dyn HirDefDB) -> AHashMap<String, Value> {
        self.resistive
            .raw
            .iter()
            .filter_map(|(unkown, val)| {
                if let SimUnkown::KirchoffLaw(node) = unkown {
                    let name = db.node_data(*node).name.to_string();
                    Some((name, *val))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn reactive_entries(&self, db: &dyn HirDefDB) -> AHashMap<String, Value> {
        self.reactive
            .raw
            .iter()
            .filter_map(|(unkown, val)| {
                if let SimUnkown::KirchoffLaw(node) = unkown {
                    let name = db.node_data(*node).name.to_string();
                    Some((name, *val))
                } else {
                    None
                }
            })
            .collect()
    }
}
