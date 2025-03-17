use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::gates::noop::NoopGate;
use plonky2::iop::target::Target;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{Poseidon2BN254Config, PoseidonGoldilocksConfig};
use plonky2_field::goldilocks_field::GoldilocksField;

/// An example of using Plonky2 to prove a proof in-circuit
/// and with P number of public inputs
/// uses the `PublicInputGateV2` which doesn't hash the public input
fn main() -> Result<()> {
    const D: usize = 2;
    type C1 = PoseidonGoldilocksConfig;
    type C2 = Poseidon2BN254Config;
    type F = GoldilocksField;

    //---------- inner layer ------------

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    const S: usize = 5;
    let num_dummy_gates = (1 << (S - 1)) + 1;
    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    const P: usize = 150;
    // The public inputs
    let mut t_list= vec![];
    for _ in 0..P {
        let t = builder.add_virtual_public_input();
        t_list.push(t);
    }

    // Provide initial values.
    let mut pw = PartialWitness::new();
    for i in 0..P {
        pw.set_target(t_list[i], F::ZERO)?;
    }

    let data = builder.build::<C1>();
    println!("inner layer: circuit size = {}", data.common.degree_bits());

    let proof = data.prove(pw)?;
    println!("inner layer: num pi = {}", proof.public_inputs.len());

    assert!(data.verify(proof.clone()).is_ok());

    //------------ outer layer ----------------

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let proof_t = builder.add_virtual_proof_with_pis(&data.common);
    let verifier_data_t = builder.add_virtual_verifier_data(builder.config.fri_config.cap_height);
    builder.verify_proof::<C1>(&proof_t, &verifier_data_t, &data.common);

    let mut t_list: Vec<Target> = vec![];

    for (i,pi) in proof_t.public_inputs.iter().enumerate() {
        let t = builder.add_virtual_public_input();
        builder.connect(*pi,t.clone());
        t_list.push(t);
    }

    // Provide initial values.
    let mut pw = PartialWitness::new();
    for i in 0..P {
        pw.set_target(t_list[i], F::ZERO)?;
    }
    pw.set_proof_with_pis_target(&proof_t,&proof);
    pw.set_verifier_data_target(&verifier_data_t,&data.verifier_only);

    let data = builder.build_unhashed_pi::<C1>();
    println!("outer layer: circuit size = {}", data.common.degree_bits());

    let proof = data.prove_unhashed_pi(pw)?;
    println!("outer layer: num pi = {}", proof.public_inputs.len());

    assert!(data.verify_unhashed_pi(proof.clone()).is_ok());

    Ok(())
}
