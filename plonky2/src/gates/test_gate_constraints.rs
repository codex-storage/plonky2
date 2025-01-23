
// used for debugging the constraint equations

use crate::field::extension::{Extendable, FieldExtension};
use crate::field::types::Field;

use crate::hash::hash_types::{HashOut, RichField};

use crate::plonk::circuit_data::{CircuitConfig};
use crate::plonk::vars::{EvaluationVars};

use crate::gates::gate::Gate;

use crate::gates::arithmetic_extension::*;
use crate::gates::base_sum::*;
use crate::gates::coset_interpolation::*;
use crate::gates::exponentiation::*;
use crate::gates::multiplication_extension::*;
use crate::gates::random_access::*;
use crate::gates::poseidon::*;
use crate::gates::poseidon_mds::*;
use crate::gates::reducing::*;
use crate::gates::reducing_extension::*;

fn make_fext<F: RichField + Extendable<D>, const D: usize>( x: u64 ) -> F::Extension {
  let mut vec: [F; D] = [F::ZERO; D];
  vec[0] = F::from_canonical_u64(x);
  vec[1] = F::from_canonical_u64(13);
  F::Extension::from_basefield_array(vec)
} 

pub fn test_gate_constraints<F: RichField + Extendable<D>, const D: usize>() {

  // create some fixed evaluation vars
  let loc_constants = 
        [ make_fext::<F,D>(666)
        , make_fext::<F,D>(77) 
        ];
  let input_hash = HashOut{ elements: 
        [ F::from_canonical_u64(101) 
        , F::from_canonical_u64(102)
        , F::from_canonical_u64(103)
        , F::from_canonical_u64(104) 
        ] };
  let loc_wires: [F::Extension; 135] = (0..135).map( |i| make_fext::<F,D>(1001 + 71*i) ).collect::<Vec<_>>().try_into().unwrap();

  let vars = EvaluationVars
        { local_constants:    &loc_constants
        , local_wires:        &loc_wires    
        , public_inputs_hash: &input_hash   
        };

  let circuit_config: CircuitConfig = CircuitConfig::standard_recursion_config();

  println!("\n------------------------------------------------------------------");
  println!("ArithmeticExtensionGate");
  let arith_ext_gate: ArithmeticExtensionGate<D> = ArithmeticExtensionGate::new_from_config(&circuit_config);
  println!("{:?}", arith_ext_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  let base_sum_gate_2: BaseSumGate<2> = BaseSumGate::new(13);
  let base_sum_gate_3: BaseSumGate<3> = BaseSumGate::new(13);
  println!("\nBaseSumGate (radix = 2)");
  println!("{:?}", base_sum_gate_2.eval_unfiltered(vars) );
  println!("\nBaseSumGate (radix = 3)");
  println!("{:?}", base_sum_gate_3.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  let coset_gate_3: CosetInterpolationGate<F,D> = CosetInterpolationGate::with_max_degree(3,8);
  let coset_gate_4: CosetInterpolationGate<F,D> = CosetInterpolationGate::with_max_degree(4,8);
  let coset_gate_5: CosetInterpolationGate<F,D> = CosetInterpolationGate::with_max_degree(5,8);
  println!("\nCosetInterpolationGate (num_bits = 3)");
  println!("{:?}", coset_gate_3.eval_unfiltered(vars) );
  println!("\nCosetInterpolationGate (num_bits = 4)");
  println!("{:?}", coset_gate_4.eval_unfiltered(vars) );
  println!("\nCosetInterpolationGate (num_bits = 5)");
  println!("{:?}", coset_gate_5.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("ExponentiationGate (13)");
  let exp_gate: ExponentiationGate<F,D> = ExponentiationGate::new(13);
  println!("{:?}", exp_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("MulExtensionGate");
  let mul_ext_gate: MulExtensionGate<D> = MulExtensionGate::new_from_config(&circuit_config);
  println!("{:?}", mul_ext_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("PosideonGate");
  let pos_gate: PoseidonGate<F,D> = PoseidonGate::new();
  println!("{:?}", pos_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("PosideonMdsGate");
  let mds_gate: PoseidonMdsGate<F,D> = PoseidonMdsGate::new();
  println!("{:?}", mds_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("RandomAccessGate");
  let ra_gate: RandomAccessGate<F,D> = RandomAccessGate::new_from_config(&circuit_config, 4);
  println!("{:?}", ra_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("ReducingGate");
  let red_gate: ReducingGate<D> = ReducingGate::new(13);
  println!("{:?}", red_gate.eval_unfiltered(vars) );

  println!("\n------------------------------------------------------------------");
  println!("ReducingExtensionGate");
  let red_ext_gate: ReducingExtensionGate<D> = ReducingExtensionGate::new(13);
  println!("{:?}", red_ext_gate.eval_unfiltered(vars) );

}