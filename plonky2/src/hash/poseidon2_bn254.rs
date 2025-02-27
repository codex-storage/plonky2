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
use ark_ff::{BigInt, PrimeField, Zero};
use rust_bn254_hash::hash::Hash;
use rust_bn254_hash::sponge::{sponge_felts_no_pad, sponge_felts_pad};

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
        let bn_out = state.squeeze();
        let bn_bytes: Vec<u8> = bn_out.iter().flat_map(|e| felts_to_bytes(e)).collect();
        let goldilocks_felts: Vec<F> = bytes_to_u64(&bn_bytes).iter().map(|e| F::from_canonical_u64(*e)).collect();
        assert!(goldilocks_felts.len()>0);
        goldilocks_felts
    }
}

// --------- Conversion helper functions ---------------------

/// converts a vec of goldilocks to bn254
/// takes 7 goldilocks and converts to 2 bn254
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
        // check that we don't push zero field elements
        if a != BN254Fr::zero() {
            result.push(a);
        }
        if b != BN254Fr::zero() {
            result.push(b);
        }
    }
    result
}

const BIGINT_TWO_TO_64:  BigInt<4> = BigInt( [0,1,0,0] );
const BIGINT_TWO_TO_128: BigInt<4> = BigInt( [0,0,1,0] );
const BIGINT_TWO_TO_192: BigInt<4> = BigInt( [0,0,0,1] );

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

/// converts a slice of bytes to 64 by taking 63 bits at a time
/// this makes it safe for conversion from bytes to Goldilocks field elems
/// this fn ignores any remaining bit that are less than 63 bits at the end
pub fn bytes_to_u64(x: &[u8]) -> Vec<u64> {
    let total_bits = x.len() * 8;
    let num_chunks = total_bits / 63; // ignore any leftover bits
    let mut result = Vec::with_capacity(num_chunks);

    for i in 0..num_chunks {
        let bit_offset = i * 63;
        let first_byte = bit_offset / 8;
        let shift = bit_offset % 8;
        // how many bits do we need? We need (shift + 63) bits in total.
        // convert that to bytes by rounding up.
        let needed_bytes = ((shift + 63) + 7) / 8;

        if first_byte + needed_bytes > x.len() {
            break; // break out if incomplete chunk
        }

        let mut chunk: u128 = 0;
        for j in 0..needed_bytes {
            chunk |= (x[first_byte + j] as u128) << (8 * j);
        }
        // shift right with `shift` bits, then mask 63 bits.
        let value = (chunk >> shift) & ((1u128 << 63) - 1);
        result.push(value as u64);
    }

    result
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

pub fn felts_to_bytes<E>(f: &E) -> Vec<u8> where
    E: CanonicalSerialize
{
    let mut bytes = Vec::new();
    f.serialize_uncompressed(&mut bytes).expect("serialization failed");
    bytes
}

pub fn bytes_to_felts<E>(bytes: &[u8]) -> E where
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
