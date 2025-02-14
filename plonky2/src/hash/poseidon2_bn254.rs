#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::mem::size_of;

use rust_bn254_hash::hash::Hash;
use crate::hash::hash_types::{BytesHash, RichField};
use crate::hash::hashing::PlonkyPermutation;
use crate::plonk::config::Hasher;
use rust_bn254_hash::sponge::{sponge_u64_pad, sponge_u64_no_pad};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

pub const SPONGE_RATE: usize = 8;
pub const SPONGE_CAPACITY: usize = 4;
pub const SPONGE_WIDTH: usize = SPONGE_RATE + SPONGE_CAPACITY;

#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Poseidon2BN254Permutation<F: RichField> {
    state: [F; SPONGE_WIDTH],
}

impl<F: RichField> Eq for Poseidon2BN254Permutation<F> {}

impl<F: RichField> AsRef<[F]> for Poseidon2BN254Permutation<F> {
    fn as_ref(&self) -> &[F] {
        &self.state
    }
}

impl<F: RichField> PlonkyPermutation<F> for Poseidon2BN254Permutation<F> {
    const RATE: usize = SPONGE_RATE;
    const WIDTH: usize = SPONGE_WIDTH;

    fn new<I: IntoIterator<Item = F>>(elts: I) -> Self {
        let mut perm = Self {
            state: [F::default(); SPONGE_WIDTH],
        };
        perm.set_from_iter(elts, 0);
        perm
    }

    fn set_elt(&mut self, elt: F, idx: usize) {
        self.state[idx] = elt;
    }

    fn set_from_slice(&mut self, elts: &[F], start_idx: usize) {
        let begin = start_idx;
        let end = start_idx + elts.len();
        self.state[begin..end].copy_from_slice(elts);
    }

    fn set_from_iter<I: IntoIterator<Item = F>>(&mut self, elts: I, start_idx: usize) {
        for (s, e) in self.state[start_idx..].iter_mut().zip(elts) {
            *s = e;
        }
    }

    fn permute(&mut self) {
        // convert state of Goldilocks elems to u64
        let mut state_u64 = vec![0u64; SPONGE_WIDTH ];
        for i in 0..SPONGE_WIDTH {
            state_u64[i]
                = self.state[i].to_canonical_u64();
        }

        // Create an iterator that repeatedly applies the sponge permutation.
        let hash_onion = core::iter::repeat_with(|| {
            // Compute the next hash layer.
            let hash = sponge_u64_no_pad(Hash::Poseidon2, state_u64.clone());
            // Convert the sponge output to u64.
            let output = felts_to_u64(hash);
            // Update the state for the next iteration.
            state_u64 = output.clone();
            output.into_iter()
        }).flatten();

        // Parse field elements from u64 stream, using rejection sampling such that words that don't
        // fit in F are ignored.
        let new_state: Vec<F> = hash_onion
            .filter(|&word| word < F::ORDER)
            .map(F::from_canonical_u64)
            .take(SPONGE_WIDTH)
            .collect();
        // update the state
        self.state = new_state.try_into().expect("State length mismatch");
    }

    fn squeeze(&self) -> &[F] {
        &self.state[..Self::RATE]
    }
}

const N: usize  = 32;
/// Keccak-256 hash function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Poseidon2BN254;
impl<F: RichField> Hasher<F> for Poseidon2BN254 {
    const HASH_SIZE: usize = N;
    type Hash = BytesHash<N>;
    type Permutation = Poseidon2BN254Permutation<F>;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        let mut state_u64 = vec![0u64; input.len() ];
        for i in 0..input.len() {
            state_u64[i]
                = input[i].to_canonical_u64();
        }
        let mut arr = [0; N];
        let hash = sponge_u64_no_pad(Hash::Poseidon2, state_u64);
        let hash_bytes = felts_to_bytes(hash);
        arr.copy_from_slice(&hash_bytes[..N]);
        BytesHash(arr)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        let mut input_bytes = vec![0; N * 2];
        input_bytes[0..N].copy_from_slice(&left.0);
        input_bytes[N..].copy_from_slice(&right.0);
        let mut arr = [0; N];
        let state_u64: Vec<u64> = input_bytes
            .chunks_exact(8)
            .map(|chunk| u64::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        let hash = sponge_u64_no_pad(Hash::Poseidon2, state_u64);
        let hash_bytes = felts_to_bytes(hash);
        arr.copy_from_slice(&hash_bytes[..N]);
        BytesHash(arr)
    }
}

fn felts_to_bytes<E>(f: E) -> Vec<u8> where
    E: CanonicalSerialize
{
    let mut bytes = Vec::new();
    f.serialize_uncompressed(&mut bytes).expect("serialization failed");
    bytes
}

fn bytes_to_felts<E>(bytes: &[u8]) -> E where
    E: CanonicalDeserialize
{
    let fr_res = E::deserialize_uncompressed(bytes).unwrap();
    fr_res
}

fn felts_to_u64<E>(f: E) -> Vec<u64>
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
