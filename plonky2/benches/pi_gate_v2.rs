use std::any::type_name;
use anyhow::{anyhow, Result};
use criterion::{criterion_group, criterion_main, Criterion};
use plonky2::gates::noop::NoopGate;
use plonky2::hash::hash_types::RichField;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::CircuitConfig;
use plonky2::plonk::config::{GenericConfig, Poseidon2BN254Config, PoseidonGoldilocksConfig};
use plonky2_field::extension::Extendable;
use plonky2_field::goldilocks_field::GoldilocksField;

/// Benchmark for building, proving, and verifying the Plonky2 circuit.
fn bench_circuit<F: RichField + Extendable<D>, const D:usize, C: GenericConfig<D, F = F>,>(c: &mut Criterion, circuit_size: usize, num_of_pi: usize) -> Result<()>{

    // Create the circuit configuration
    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    let num_dummy_gates = match circuit_size {
        0 => return Err(anyhow!("size must be at least 1")),
        1 => 0,
        2 => 1,
        n => (1 << (n - 1)) + 1,
    };

    for _ in 0..num_dummy_gates {
        builder.add_gate(NoopGate, vec![]);
    }

    // The public inputs
    let mut t_list= vec![];
    for _ in 0..num_of_pi {
        let t = builder.add_virtual_public_input();
        t_list.push(t);
    }

    // Benchmark Group
    let mut group = c.benchmark_group(format!("Circuit size = {}, pi size = {}, and Hasher = {}", circuit_size, num_of_pi, type_name::<C::Hasher>()));

    // Benchmark the Circuit Building Phase
    group.bench_function("Build Circuit", |b| {
        b.iter(|| {
            let config = CircuitConfig::standard_recursion_config();
            let mut local_builder = CircuitBuilder::<F, D>::new(config);
            for _ in 0..num_dummy_gates {
                local_builder.add_gate(NoopGate, vec![]);
            }
            let _data = local_builder.build_unhashed_pi::<C>();
        })
    });

    let data = builder.build_unhashed_pi::<C>();
    println!("Circuit size (degree bits): {:?}", data.common.degree_bits());

    // Create a PartialWitness
    let mut pw = PartialWitness::new();
    for i in 0..num_of_pi {
        pw.set_target(t_list[i], F::ZERO)?;
    }

    // Benchmark the Proving Phase
    group.bench_function("Prove Circuit", |b| {
        b.iter(|| {
            let local_pw = pw.clone();
            data.prove_unhashed_pi(local_pw).expect("Failed to prove circuit")
        })
    });

    // Generate the proof once for verification benchmarking
    let proof_with_pis = data.prove_unhashed_pi(pw.clone()).expect("Failed to prove circuit");
    let verifier_data = data.verifier_data();

    println!("Proof size: {} bytes", proof_with_pis.to_bytes().len());
    println!("num of pi = {}", proof_with_pis.public_inputs.len());

    // Benchmark the Verifying Phase
    group.bench_function("Verify Proof", |b| {
        b.iter(|| {
            verifier_data.verify_unhashed_pi(proof_with_pis.clone()).expect("Failed to verify proof");
        })
    });

    group.finish();
    Ok(())
}

fn bench_multiple_pi_values(c: &mut Criterion){
    const D: usize = 2;
    type C1 = PoseidonGoldilocksConfig;
    type C2 = Poseidon2BN254Config;
    type F = GoldilocksField;

    // bench_circuit::<F,D,C1>(c, 12, 10).expect("failed");
    // bench_circuit::<F,D,C2>(c, 12, 10).expect("failed");
    //
    // bench_circuit::<F,D,C1>(c, 12, 50).expect("failed");
    // bench_circuit::<F,D,C2>(c, 12, 50).expect("failed");
    //
    // bench_circuit::<F,D,C1>(c, 12, 100).expect("failed");
    // bench_circuit::<F,D,C2>(c, 12, 100).expect("failed");

    // bench_circuit::<F,D,C1>(c, 12, 500).expect("failed");
    // bench_circuit::<F,D,C2>(c, 12, 500).expect("failed");

    bench_circuit::<F,D,C1>(c, 12, 1000).expect("failed");
    bench_circuit::<F,D,C2>(c, 12, 1000).expect("failed");
}

/// Criterion benchmark group
criterion_group!{
    name = prove_verify_benches;
    config = Criterion::default().sample_size(10);
    targets = bench_multiple_pi_values
}
criterion_main!(prove_verify_benches);
