// use std::fs;
use anyhow::Result;
use plonky2::gates::noop::NoopGate;
use plonky2::iop::witness::PartialWitness;
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, Poseidon2BN254Config};
use plonky2::plonk::prover::DEFAULT_PROVER_OPTIONS;
use plonky2::plonk::verifier::{HashStatisticsPrintLevel, VerifierOptions};

/// An example of using Plonky2 over BN254 to a dummy circuit of size S.
fn main() -> Result<()> {
    const D: usize = 2;
    type C = Poseidon2BN254Config;
    type F = <C as GenericConfig<D>>::F;

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);
    const S: usize = 5;
    let num_dummy_gates = (1 << (S - 1)) + 1;
    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    let pw = PartialWitness::new();

    let data = builder.build::<C>();
    println!("circ size = {}", data.common.degree_bits());

    let prover_opts = DEFAULT_PROVER_OPTIONS;

    println!("proving ...");

    let proof = data.prove_with_options(pw, &prover_opts)?;

    // serialize circuit into JSON
    // let common_circuit_data_serialized        = serde_json::to_string(&data.common       ).unwrap();
    // let verifier_only_circuit_data_serialized = serde_json::to_string(&data.verifier_only).unwrap();
    // let proof_serialized                      = serde_json::to_string(&proof             ).unwrap();
    // fs::write("bn_common.json" , common_circuit_data_serialized)       .expect("Unable to write file");
    // fs::write("bn_vkey.json"   , verifier_only_circuit_data_serialized).expect("Unable to write file");
    // fs::write("bn_proof.json"  , proof_serialized)                     .expect("Unable to write file");

    let verifier_opts = VerifierOptions {
        print_hash_statistics: HashStatisticsPrintLevel::Summary,
        hash_public_input: true,
    };

    assert!(data.verify_with_options(proof, &verifier_opts).is_ok());

    Ok(())
}
