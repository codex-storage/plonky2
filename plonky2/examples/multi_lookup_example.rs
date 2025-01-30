
// example using 2 different lookup tables

#![allow(unused_imports)]
#![allow(dead_code)]

use std::fs;

use anyhow::Result;
use plonky2::field::types::Field;
use plonky2::iop::witness::{PartialWitness, WitnessWrite};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::circuit_data::{CircuitConfig,CircuitData};
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
use plonky2::plonk::prover::ProverOptions;
use plonky2::plonk::verifier::{VerifierOptions, HashStatisticsPrintLevel};

//use plonky2::gadgets::lookup;

use env_logger;

pub const TABLE_SIZE_1: usize = 128;
pub const TABLE_SIZE_2: usize = 256;

fn main() -> Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    let nn = 50;
    let aa = 2;
    let bb = 7;

    env_logger::init();

    let config = CircuitConfig::standard_recursion_config();
    let mut builder = CircuitBuilder::<F, D>::new(config);

    // generate the two lookup tables
    let indices1: [u16; TABLE_SIZE_1] = core::array::from_fn( |i| i as u16 );
    let indices2: [u16; TABLE_SIZE_2] = core::array::from_fn( |i| i as u16 );

    let values1: [u16; TABLE_SIZE_1] = indices1.iter().map( |x| x+100  ).collect::<Vec<u16>>().try_into().unwrap();
    let values2: [u16; TABLE_SIZE_2] = indices2.iter().map( |x| x+1000 ).collect::<Vec<u16>>().try_into().unwrap();

    let my_lut1_index = builder.add_lookup_table_from_table( &indices1 , &values1 );
    let my_lut2_index = builder.add_lookup_table_from_table( &indices2 , &values2 );

    println!("LUT#1 size={} index={}",indices1.len(), my_lut1_index);
    println!("LUT#2 size={} index={}",indices2.len(), my_lut2_index);

    // The arithmetic circuit.
    let the_a    = builder.add_virtual_target();
    let the_b    = builder.add_virtual_target();
    let mut sum_seq  = builder.constant(F::ZERO);
    let mut sum_val1 = builder.constant(F::ZERO);
    let mut sum_val2 = builder.constant(F::ZERO);
    for i in 0..nn {

        let mult  = builder.constant(F::from_canonical_usize(i));
        let this  = builder.mul_add(mult, the_a, the_b);                   // x*y + z
       
        let this2 = builder.add_const(this, F::from_canonical_usize(64));  // this+64

        sum_seq = builder.add(sum_seq, this);

        let out1 = builder.add_lookup_from_index( this, my_lut1_index );
        sum_val1 = builder.add(sum_val1, out1);

        let out2 = builder.add_lookup_from_index( this2, my_lut2_index );
        sum_val2 = builder.add(sum_val2, out2);

    }

    // Public inputs are the two initial values (provided below) and the result (which is generated).
    builder.register_public_input(the_a);
    builder.register_public_input(the_b);
    builder.register_public_input(sum_seq);
    builder.register_public_input(sum_val1);
    builder.register_public_input(sum_val2);

    // Provide initial values.
    let mut pw = PartialWitness::new();
    pw.set_target(the_a, F::from_canonical_usize(aa))?;
    pw.set_target(the_b, F::from_canonical_usize(bb))?;

    let data: CircuitData<F,C,D> = builder.build::<C>();

    let prover_opts = ProverOptions {
        export_witness: Some(String::from("multi_lookup_witness.json")),
        print_hash_statistics: HashStatisticsPrintLevel::None,
    };
    let proof = data.prove_with_options(pw, &prover_opts)?;

    // serialize circuit into JSON
    let common_circuit_data_serialized        = serde_json::to_string(&data.common       ).unwrap();
    let verifier_only_circuit_data_serialized = serde_json::to_string(&data.verifier_only).unwrap();
    let proof_serialized                      = serde_json::to_string(&proof             ).unwrap();
    fs::write("multi_lookup_common.json" , common_circuit_data_serialized)       .expect("Unable to write file");
    fs::write("multi_lookup_vkey.json"   , verifier_only_circuit_data_serialized).expect("Unable to write file");
    fs::write("multi_lookup_proof.json"  , proof_serialized)                     .expect("Unable to write file");

    println!("the arithmetic progression is: `q[i] := {}*i + {}` for 0<=i<{}",proof.public_inputs[0],proof.public_inputs[1],nn);
    println!("sum of the progression   = {}",proof.public_inputs[2]);
    println!("sum of lookups in LUT #1 = {}",proof.public_inputs[3]);
    println!("sum of lookups in LUT #2 = {}",proof.public_inputs[4]);
    
    let res = data.verify(proof);

    res
}
