use std::fs;
use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::gates::noop::NoopGate;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{Poseidon2BN254Config, PoseidonGoldilocksConfig};
use plonky2::plonk::prover::ProverOptions;
use plonky2::plonk::verifier::{HashStatisticsPrintLevel, VerifierOptions};
use plonky2_field::goldilocks_field::GoldilocksField;

/// An example of using Plonky2 to prove a circuit with size S
/// and with P number of public inputs
/// uses the `PublicInputGateV2` which doesn't hash the public input
fn main() -> Result<()> {
    const D: usize = 2;

    type C1 = PoseidonGoldilocksConfig;
    type C2 = Poseidon2BN254Config;
    type F = GoldilocksField;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    const S: usize = 5;
    let num_dummy_gates = (1 << (S - 1)) + 1;
    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    const P: usize = 1000;
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

    let data = builder.build_unhashed_pi::<C1>();
    println!("circuit size = {}", data.common.degree_bits());

    let prover_opts = ProverOptions {
        export_witness: Some(String::from("pi_gate_v2_witness.json")),
        print_hash_statistics: HashStatisticsPrintLevel::Info,
        hash_public_input: false,
    };

    let proof = data.prove_with_options(pw, &prover_opts)?;
    println!("num pi = {}", proof.public_inputs.len());

    let verifier_opts = VerifierOptions {
        print_hash_statistics: HashStatisticsPrintLevel::Summary,
        hash_public_input: false,
    };

    let common_circuit_data_serialized        = serde_json::to_string(&data.common).unwrap();
    let verifier_only_circuit_data_serialized = serde_json::to_string(&data.verifier_only).unwrap();
    let proof_serialized                      = serde_json::to_string(&proof).unwrap();
    fs::write("pi_gate_v2_common.json", common_circuit_data_serialized        ).expect("Unable to write file");
    fs::write("pi_gate_v2_vkey.json"  , verifier_only_circuit_data_serialized ).expect("Unable to write file");
    fs::write("pi_gate_v2_proof.json" , proof_serialized                      ).expect("Unable to write file");


    assert!(data.verify_with_options(proof, &verifier_opts).is_ok());

    Ok(())
}
