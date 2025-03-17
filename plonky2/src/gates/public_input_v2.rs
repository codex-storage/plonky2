#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};
use core::ops::Range;

use crate::field::extension::Extendable;
use crate::field::packed::PackedField;
use crate::gates::gate::Gate;
use crate::gates::packed_util::PackedEvaluableBase;
use crate::gates::util::StridedConstraintConsumer;
use crate::hash::hash_types::RichField;
use crate::iop::ext_target::ExtensionTarget;
use crate::iop::generator::WitnessGeneratorRef;
use crate::plonk::circuit_builder::CircuitBuilder;
use crate::plonk::circuit_data::CommonCircuitData;
use crate::plonk::vars::{
    EvaluationTargets, EvaluationVars, EvaluationVarsBase, EvaluationVarsBaseBatch,
    EvaluationVarsBasePacked,
};
use crate::util::serialization::{Buffer, IoResult, Read, Write};

/// A gate which enforces that each wire matches a corresponding public-input element.
///
/// Specifically, if this gate has `num_pub_inputs` wires, then for each wire i in
/// [0..num_pub_inputs):
///
///   local_wires[i] == public_inputs[i]
///
/// If the circuit has more public inputs than the circuit config's `num_wires` you'll need multiple gates
#[derive(Debug)]
pub struct PublicInputGateV2 {
    /// How many public inputs are enforced by this gate.
    pub num_pub_inputs: usize,
    /// start index from which we take the public input
    pub index: usize,
}

impl PublicInputGateV2 {

    /// careful with this fn, you must ensure `num_pub_inputs` <= the circuit config's `num_wires`.
    pub fn new(num_pub_inputs: usize, index: usize) -> Self {
        Self {
            num_pub_inputs,
            index
        }
    }
}

impl<F: RichField + Extendable<D>, const D: usize> Gate<F, D> for PublicInputGateV2 {
    fn id(&self) -> String {
        "PublicInputGateV2".into()
    }

    fn short_id(&self) -> String {
        "PublicInputGateV2".into()
    }

    fn serialize(
        &self,
        dst: &mut Vec<u8>,
        _common_data: &CommonCircuitData<F, D>,
    ) -> IoResult<()> {
        dst.write_usize(self.num_pub_inputs)?;
        dst.write_usize(self.index)
    }

    fn deserialize(src: &mut Buffer, _common_data: &CommonCircuitData<F, D>) -> IoResult<Self> {
        let num_pub_inputs = src.read_usize()?;
        let index = src.read_usize()?;

        Ok(Self {
            num_pub_inputs,
            index
        })
    }

    fn eval_unfiltered(&self, vars: EvaluationVars<F, D>) -> Vec<F::Extension> {
        // For each i in [0..num_pub_inputs], the constraint is: local_wires[i] - public_inputs[i]
        // That must be 0 if the public input is correct.
        (0..self.num_pub_inputs)
            .map(|i| {
                let wire_value = vars.local_wires[i];
                let pi_value = vars.public_inputs[self.index + i].into();
                wire_value - pi_value
            })
            .collect()
    }

    fn eval_unfiltered_base_one(
        &self,
        _vars: EvaluationVarsBase<F>,
        _yield_constr: StridedConstraintConsumer<F>,
    ) {
        panic!("use eval_unfiltered_base_packed instead");
    }

    fn eval_unfiltered_base_batch(&self, vars_base: EvaluationVarsBaseBatch<F>) -> Vec<F> {
        self.eval_unfiltered_base_batch_packed(vars_base)
    }

    fn eval_unfiltered_circuit(
        &self,
        builder: &mut CircuitBuilder<F, D>,
        vars: EvaluationTargets<D>,
    ) -> Vec<ExtensionTarget<D>> {
        todo!()
        // // Circuit-level version of the same logic.
        // (0..self.num_pub_inputs)
        //     .map(|i| {
        //         // local_wires[i] - public_inputs[i]
        //         let pi_part_ex = builder.convert_to_ext(vars.public_inputs[self.index + i]);
        //         builder.sub_extension(vars.local_wires[i], pi_part_ex)
        //     })
        //     .collect()
    }

    fn generators(&self, _row: usize, _local_constants: &[F]) -> Vec<WitnessGeneratorRef<F, D>> {
        Vec::new()
    }

    fn num_wires(&self) -> usize {
        self.num_pub_inputs
    }

    fn num_constants(&self) -> usize {
        0
    }

    fn degree(&self) -> usize {
        1
    }

    fn num_constraints(&self) -> usize {
        self.num_pub_inputs
    }
}

impl<F: RichField + Extendable<D>, const D: usize> PackedEvaluableBase<F, D> for PublicInputGateV2 {
    fn eval_unfiltered_base_packed<P: PackedField<Scalar = F>>(
        &self,
        vars: EvaluationVarsBasePacked<P>,
        mut yield_constr: StridedConstraintConsumer<P>,
    ) {
        yield_constr.many(
            (0..self.num_pub_inputs).map(|i| vars.local_wires[i] - vars.public_inputs[self.index + i]),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks_field::GoldilocksField;
    use crate::gates::gate_testing::{test_eval_fns, test_low_degree};
    use crate::gates::public_input_v2::PublicInputGateV2;
    use crate::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    #[test]
    fn pi_v2_low_degree() {
        test_low_degree::<GoldilocksField, _, 4>(PublicInputGateV2{num_pub_inputs:4,index:0})
    }

    #[test]
    fn pi_v2_eval_fns() -> anyhow::Result<()> {
        todo!()
        // const D: usize = 2;
        // type C = PoseidonGoldilocksConfig;
        // type F = <C as GenericConfig<D>>::F;
        // test_eval_fns::<F, C, _, D>(PublicInputGateV2{num_pub_inputs:4,index:0})
    }
}
