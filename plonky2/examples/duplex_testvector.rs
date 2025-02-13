use plonky2::field::types::Field;
use plonky2::iop::challenger::*;
use plonky2::hash::hashing::*;
use plonky2::hash::poseidon::{PoseidonHash,Poseidon};
use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

/// An example of using Plonky2 to prove a statement of the form
/// "I know n * (n + 1) * ... * (n + 99)".
/// When n == 1, this is proving knowledge of 100!.
fn main() {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;
    type H =  PoseidonHash;

    let mut duplex : Challenger<F,H> = Challenger::new();

    let mut counter = 1001;
    let mut sum: F = F::ZERO;
    println!("duplexTestVector =");
    let mut c = '[';
    for a in 1..21 {
        for s in 1..21 {
            for i in 0..a { 
                duplex.observe_element(F::from_canonical_u64(counter));
                counter = counter+1;
            }
            println!("    -- #absorb = {} -> #squeeze = {}",a,s);
            for i in 0..s { 
                let out = duplex.get_challenge();
                sum = sum + out;
                println!("  {} {}",c,out);
                c = ',';
            }
        }
    }
    println!("  ]");
    println!("");
    println!("sum = {}",sum);
}
