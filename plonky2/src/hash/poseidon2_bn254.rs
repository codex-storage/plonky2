#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::fmt::Debug;
use core::mem::size_of;

use crate::hash::hash_types::{BN254HashOut, RichField};
use crate::hash::hashing::PlonkyPermutation;
use crate::plonk::config::{GenericField, Hasher};
use rust_bn254_hash::state::State;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_bn254::{Fr as BN254Fr};
use rust_bn254_hash::poseidon2::permutation::permute_inplace as permute_bn254_inplace;
use ark_ff::{ PrimeField, Zero,};
use num::Integer;
use num_bigint::BigUint;
use ark_ff::BigInt as arkBigInt;
use rust_bn254_hash::hash::Hash;
use rust_bn254_hash::sponge::{sponge_felts_no_pad, sponge_felts_pad};
use plonky2_field::goldilocks_field::GoldilocksField;
use plonky2_field::types::Field64;

pub const SPONGE_RATE: usize = 2;
pub const SPONGE_CAPACITY: usize = 1;
pub const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

/// Poseidon2 state with BN254 elements
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub struct Poseidon2BN254Perm {
    state: [BN254Fr; SPONGE_WIDTH],
}

/// needed for PlonkyPermutation
impl AsRef<[BN254Fr]> for Poseidon2BN254Perm {
    fn as_ref(&self) -> &[BN254Fr] {
        &self.state
    }
}

impl PlonkyPermutation<BN254Fr> for Poseidon2BN254Perm {
    const RATE: usize = SPONGE_RATE;
    const WIDTH: usize = SPONGE_WIDTH;

    fn new<I: IntoIterator<Item = BN254Fr>>(elts: I) -> Self {
        let mut perm = Self {
            state: [BN254Fr::default(); SPONGE_WIDTH],
        };
        perm.set_from_iter(elts, 0);
        perm
    }

    fn set_elt(&mut self, elt: BN254Fr, idx: usize) {
        self.state[idx] = elt;
    }

    fn set_from_slice(&mut self, elts: &[BN254Fr], start_idx: usize) {
        let begin = start_idx;
        let end = start_idx + elts.len();
        self.state[begin..end].copy_from_slice(elts);
    }

    fn set_from_iter<I: IntoIterator<Item = BN254Fr>>(&mut self, elts: I, start_idx: usize) {
        for (s, e) in self.state[start_idx..].iter_mut().zip(elts) {
            *s = e;
        }
    }

    /// calls the permutation in `rust-bn254-hash`
    /// we can probably refactor the state and eliminate the conversion in this fn.
    fn permute(&mut self) {
        let mut s = State{
            x: self.state[0].clone(),
            y: self.state[1].clone(),
            z: self.state[2].clone(),
        };

        permute_bn254_inplace(&mut s);

        self.state = [
            s.x,
            s.y,
            s.z,
        ];

    }

    fn squeeze(&self) -> &[BN254Fr] {
        &self.state[..Self::RATE]
    }

}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Poseidon2BN254;
impl<F: RichField> Hasher<F> for Poseidon2BN254 {
    type HF = BN254Fr;
    const HASH_SIZE: usize = 32;
    type Hash = BN254HashOut;
    type Permutation = Poseidon2BN254Perm;

    fn hash_no_pad(input: &[GenericField<F>]) -> Self::Hash {
        let bn_felts = generic_field_to_bn(input);
        let hash = sponge_felts_no_pad(Hash::Poseidon2, bn_felts);
        BN254HashOut {
            element: hash,
        }
    }

    fn hash_pad(input: &[GenericField<F>]) -> Self::Hash {
        let bn_felts = generic_field_to_bn(input);
        let hash = sponge_felts_pad(Hash::Poseidon2, bn_felts);
        BN254HashOut {
            element: hash,
        }
    }

    fn hash_or_noop(inputs: &[GenericField<F>]) -> Self::Hash {
        let hash_size = 32;
        if check_len_in_bytes(inputs) <= hash_size {
            if inputs.len() == 1 {
                // if there is one element and it is a BN field element return it.
                if let GenericField::BN254(v) = inputs[0].clone() {
                    return BN254HashOut{element: v};
                }
            }
        }
        // TODO: if we get 4 or less Goldilocks -> convert to BN return?
        Self::hash_no_pad(inputs)
    }


    fn sponge(state: &mut Self::Permutation, input: Vec<GenericField<F>>) {

        let bn_felts = generic_field_to_bn(&input);

        // absorb in overwrite mode
        for chunk in bn_felts.chunks(2) {
            state.set_from_slice(chunk, 0);
            state.permute();
        }

    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {

        let mut perm = Self::Permutation::new(core::iter::repeat(BN254Fr::zero()));
        perm.set_from_slice(&[left.element], 0);
        perm.set_from_slice(&[right.element], 1);

        perm.permute();
        let out = perm.squeeze();

        BN254HashOut {
            element: out[0].clone(),
        }
    }

    fn squeeze_goldilocks(state: &mut Self::Permutation) -> Vec<F> {
        // Squeeze out BN254 elements from the sponge state.
        let bn_out = state.squeeze();

        // convert bn to goldilocks
        bn_to_goldilocks(bn_out)
    }
}

// --------- Conversion helper functions ---------------------

/// Converts a slice of BN254 field elements to a vector of Goldilocks (F) by:
///
///  - Interpreting each BN254 element as an unsigned big integer `BigUint`.
///  - Repeatedly taking `remainder = X mod Goldilocks::ORDER` (which fits in a `u64`)
///    and then dividing `X` by `Goldilocks::ORDER`.
///  - Repeat this exactly 3 times for each BN254 element in the slice, generating 3*l Goldilocks elements
///    where l = length of the slice.
///
/// We use this primarily in hashing contexts (for Fiat-Shamir in Plonky2 circuits), where
/// we want to safely convert a ~254-bit BN254 element into multiple 64-bit
/// Goldilocks elements. The little leftover in `X` after extracting 3 remainders
/// is trashed, so there is a negligible bias.
fn bn_to_goldilocks<F: RichField>(input: &[BN254Fr]) -> Vec<F> {
    // Goldilocks order
    let r: BigUint = BigUint::from(GoldilocksField::ORDER);

    let mut goldilocks_felts = Vec::new();
    // For each BN254 field element, extract 3 Goldilocks elements.
    for fe in input.into_iter().cloned() {
        // Convert BN254Fr -> 256-bit big integer.
        let mut big: BigUint = fe.into_bigint().into();

        // We want three remainders in [0, p_Goldilocks), each fits into a 64-bit integer.
        for _ in 0..3 {

            let (quotient, remainder) = big.div_rem(&r);
            let rem_u64 = remainder.to_u64_digits();

            // check just for safety:
            if rem_u64.len() > 1 {
                panic!("Remainder unexpectedly larger than 64 bits.")
            } else if rem_u64.len() == 1{
                let r64 = rem_u64[0];
                goldilocks_felts.push(F::from_canonical_u64(r64));
            }

            // Update big to the quotient for the next remainder.
            big = quotient;
        }
    }
    goldilocks_felts
}


/// Convert a vec of Goldilocks elements into BN254 elements.
/// - pack `7` consecutive `u64` values into `2` BN254 field elements.
/// - If the total number of Goldilocks elements is not a multiple of 7, we
///   zero‐pad the last chunk up to 7. That chunk still produces 2 BN254 field elements.
/// - Returns: A `Vec<BN254Fr>`
///
/// **Note**: This is used for packing a sequence of 64-bit words into
/// BN254 in a safe way. It is NOT the inverse of `bn_to_goldilocks`
fn goldilocks_to_bn<F: RichField>(input: &Vec<F>) -> Vec<BN254Fr>{
    let u64s: Vec<u64> = input.iter().map(|x| x.to_canonical_u64()).collect();
    let l = u64s.len();
    let m = l / 7;
    let mut result = Vec::new();

    for i in 0..m {
        let group: [u64; 7] = u64s[7 * i..7 * (i + 1)].try_into().unwrap();
        let (a, b) = u64s_to_felts(group);
        result.push(a);
        result.push(b);
    }

    let r = l - 7 * m;
    if r > 0 {
        let mut ws = [0u64; 7];
        for i in 0..r {
            ws[i] = u64s[7 * m + i];
        }
        let (a, b) = u64s_to_felts(ws);
        result.push(a);
        result.push(b);
    }
    result
}

const BIGINT_TWO_TO_64:  arkBigInt<4> = arkBigInt( [0,1,0,0] );
const BIGINT_TWO_TO_128: arkBigInt<4> = arkBigInt( [0,0,1,0] );
const BIGINT_TWO_TO_192: arkBigInt<4> = arkBigInt( [0,0,0,1] );

/// converts u64 to BN254 - taken directly from: rust-bn254-hash
pub fn u64s_to_felts(ws: [u64; 7]) -> (BN254Fr, BN254Fr) {
    let hi = ws[6] >> 32;
    let lo = ws[6] & 0xFFFF_FFFF;

    let field_powers_of_two_to_64: [BN254Fr;3] =
        [
            BN254Fr::from_bigint(BIGINT_TWO_TO_64 ).unwrap(),
            BN254Fr::from_bigint(BIGINT_TWO_TO_128).unwrap(),
            BN254Fr::from_bigint(BIGINT_TWO_TO_192).unwrap()
        ];

    let x = BN254Fr::from(ws[0])
        + field_powers_of_two_to_64[0] * BN254Fr::from(ws[1])
        + field_powers_of_two_to_64[1] * BN254Fr::from(ws[2])
        + field_powers_of_two_to_64[2] * BN254Fr::from(lo);

    let y = BN254Fr::from(ws[3])
        + field_powers_of_two_to_64[0] * BN254Fr::from(ws[4])
        + field_powers_of_two_to_64[1] * BN254Fr::from(ws[5])
        + field_powers_of_two_to_64[2] * BN254Fr::from(hi);

    (x, y)
}

/// helper function: converts a slice of GenericField<F> into a Vec<BN254Fr>
/// the fn groups consecutive Goldilocks elements and converting them in one shot.
fn generic_field_to_bn<F: RichField>(input: &[GenericField<F>]) -> Vec<BN254Fr> {
    let mut bn_felts = Vec::new();
    let mut temp_goldilocks = Vec::new();

    for e in input.iter().copied() {
        match e {
            GenericField::Goldilocks(v) => {
                // accumulate consecutive Goldilocks field elems.
                temp_goldilocks.push(v);
            }
            GenericField::BN254(v) => {
                // convert any accumulated Goldilocks elems.
                if !temp_goldilocks.is_empty() {
                    let converted = goldilocks_to_bn(&temp_goldilocks);
                    bn_felts.extend(converted);
                    temp_goldilocks.clear();
                }
                // push the BN field element directly.
                bn_felts.push(v);
            }
        }
    }
    // convert any remaining Goldilocks elements.
    if !temp_goldilocks.is_empty() {
        let converted = goldilocks_to_bn(&temp_goldilocks);
        bn_felts.extend(converted);
    }

    bn_felts
}

/// computes the length in bytes of a vector of generic field elements.
fn check_len_in_bytes<F: RichField>(input: &[GenericField<F>]) -> usize{
    input.iter().map(|elem| {
        match elem {
            GenericField::BN254(_) => 32,
            GenericField::Goldilocks(_)  => 8,
        }
    }).sum()
}


//------------------ serialization for BN254 ---------------------

pub fn felts_to_bytes_le<E>(f: &E) -> Vec<u8> where
    E: CanonicalSerialize
{
    let mut bytes = Vec::new();
    f.serialize_uncompressed(&mut bytes).expect("serialization failed");
    bytes
}

pub fn bytes_le_to_felts<E>(bytes: &[u8]) -> E where
    E: CanonicalDeserialize
{
    let fr_res = E::deserialize_uncompressed(bytes).unwrap();
    fr_res
}

pub fn felts_to_u64<E>(f: E) -> Vec<u64>
    where
        E: CanonicalSerialize,
{
    let mut bytes = Vec::new();
    f.serialize_uncompressed(&mut bytes)
        .expect("serialization failed");
    bytes
        .chunks_exact(size_of::<u64>())
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr as BN254Fr;
    use ark_ff::{One, Zero};
    use ark_std::{test_rng, UniformRand};
    use plonky2_field::types::Field;

    /// Test that converting a bn254 element to bytes and back.
    #[test]
    fn test_felts_bytes_roundtrip() {
        let element = <BN254Fr as PrimeField>::from_bigint(arkBigInt::from(987654321u64)).unwrap();
        let bytes = felts_to_bytes_le(&element);
        assert_eq!(bytes.len(), 32, "Expected 32 bytes for BN254Fr serialization");
        let recovered: BN254Fr = bytes_le_to_felts(&bytes);
        assert_eq!(element, recovered, "Roundtrip conversion did not recover the original element");
    }

    /// Test roundtrip with edge cases: zero and one.
    #[test]
    fn test_zero_and_one_byte_conversion() {
        let zero = BN254Fr::zero();
        let one = BN254Fr::one();

        let zero_bytes = felts_to_bytes_le(&zero);
        let one_bytes = felts_to_bytes_le(&one);

        // Check that both serializations are 32 bytes.
        assert_eq!(zero_bytes.len(), 32, "Zero should serialize to 32 bytes");
        assert_eq!(one_bytes.len(), 32, "One should serialize to 32 bytes");

        let zero_back: BN254Fr = bytes_le_to_felts(&zero_bytes);
        let one_back: BN254Fr = bytes_le_to_felts(&one_bytes);

        assert_eq!(zero, zero_back, "Zero did not roundtrip correctly");
        assert_eq!(one, one_back, "One did not roundtrip correctly");
    }

    /// Test that bn_to_goldilocks produces exactly 3 Goldilocks per BN254.
    #[test]
    fn test_bn_to_goldilocks_three_remainders() {
        // We'll test random BN254 elements to ensure no overflow panic.
        let num_tests = 1000;
        let mut bn_vec = Vec::with_capacity(num_tests);
        for _ in 0..num_tests {
            // A random BN254 field element
            let fe = BN254Fr::rand(&mut test_rng());
            bn_vec.push(fe);
        }

        let goldi_vec = bn_to_goldilocks::<GoldilocksField>(&bn_vec);
        // Should be exactly 3 * num_tests
        assert_eq!(goldi_vec.len(), 3 * num_tests);
    }

    /// Test that exactly 7 Goldilocks produce 2 BN254, and leftover is padded for partial groups.
    #[test]
    fn test_goldilocks_to_bn_packing() {
        // 7 exact Goldilocks => 2 BN254
        let goldis7 = vec![
            GoldilocksField::from_canonical_u64(1),
            GoldilocksField::from_canonical_u64(2),
            GoldilocksField::from_canonical_u64(3),
            GoldilocksField::from_canonical_u64(4),
            GoldilocksField::from_canonical_u64(5),
            GoldilocksField::from_canonical_u64(6),
            GoldilocksField::from_canonical_u64(7),
        ];
        let bn_out = goldilocks_to_bn(&goldis7);
        assert_eq!(bn_out.len(), 2, "7 Goldilocks should map to 2 BN254 elements");

        // Now test leftover: 8 Goldilocks => we expect 2 BN254 from the first 7, plus
        // 2 more BN254 for the leftover 1 (padded to 7). So total 4 BN254 elements.
        let goldis8 = {
            let mut v = goldis7.clone();
            v.push(GoldilocksField::from_canonical_u64(123));
            v
        };
        let bn_out_8 = goldilocks_to_bn(&goldis8);
        assert_eq!(bn_out_8.len(), 4, "8 Goldilocks -> 4 BN254");
    }
}

