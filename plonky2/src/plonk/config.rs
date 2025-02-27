//! Hashing configuration to be used when building a circuit.
//!
//! This module defines a [`Hasher`] trait as well as its recursive
//! counterpart [`AlgebraicHasher`] for in-circuit hashing. It also
//! provides concrete configurations, one fully recursive leveraging
//! the Poseidon hash function both internally and natively, and one
//! mixing Poseidon internally and truncated Keccak externally.

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use core::fmt::{Debug};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::field::extension::quadratic::QuadraticExtension;
use crate::field::extension::{Extendable, FieldExtension};
use crate::field::goldilocks_field::GoldilocksField;
use crate::hash::hash_types::{HashOut, RichField};
use crate::hash::hashing::PlonkyPermutation;
use crate::hash::keccak::KeccakHash;
use crate::hash::poseidon::PoseidonHash;
use crate::iop::target::{BoolTarget, Target};
use crate::plonk::circuit_builder::CircuitBuilder;
use ark_bn254::Fr as BN254Fr;
use ark_ff::{One, Zero};
use crate::hash::poseidon2_bn254::{bytes_to_felts, felts_to_bytes, Poseidon2BN254};

pub trait GenericHashOut<F: RichField>:
Copy + Clone + Debug + Eq + PartialEq + Send + Sync + Serialize + DeserializeOwned
{
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;

    fn to_vec(&self) -> Vec<GenericField<F>>;
}

/// generic field enum - supports only 2 fields for now
/// Supported fields: Goldilocks , BN254 Fr (from Arkworks)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GenericField<F: RichField> {
    Goldilocks(F),
    BN254(BN254Fr),
}

/// hasher field trait to cover fields in `GenericField` enum
pub trait HasherField: Default + Sized + Copy + Debug + Eq + PartialEq + Sync + Send {
    fn get_one() -> Self;
    fn get_zero() -> Self;
    fn to_bytes(&self) -> Vec<u8>;

    fn from_bytes(b: &[u8]) -> Self;

}

/// BN254 as Hasherfield
impl HasherField for BN254Fr {
    fn get_one() -> Self {
        BN254Fr::one()
    }

    fn get_zero() -> Self {
        BN254Fr::zero()
    }

    fn to_bytes(&self) -> Vec<u8> {
        felts_to_bytes::<BN254Fr>(&self)
    }

    fn from_bytes(b: &[u8]) -> Self {
        bytes_to_felts::<BN254Fr>(b)
    }
}

/// RichField (Goldilocks) as Hasherfield
impl <T: RichField> HasherField for T {
    fn get_one() -> Self {
        T::ONE
    }

    fn get_zero() -> Self {
        T::ZERO
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_canonical_u64().to_le_bytes().to_vec()
    }

    fn from_bytes(b: &[u8]) -> Self {
        assert_eq!(b.len(), 8, "Input vector must have exactly 8 bytes");
        let arr: [u8; 8] = b.try_into().expect("Conversion to array failed");
        let element = u64::from_le_bytes(arr);
        T::from_canonical_u64(element)
    }
}

/// Trait for hash functions.
pub trait Hasher<F: RichField>: Sized + Copy + Debug + Eq + PartialEq {

    type HF: HasherField;

    /// Size of `Hash` in bytes.
    const HASH_SIZE: usize;

    /// Hash Output
    type Hash: GenericHashOut<F>;

    /// Permutation used in the sponge construction.
    type Permutation: PlonkyPermutation<Self::HF>;

    /// Hash a message without any padding step. Note that this can enable length-extension attacks.
    /// However, it is still collision-resistant in cases where the input has a fixed length.
    fn hash_no_pad(input: &[GenericField<F>]) -> Self::Hash;

    /// Pad the message using the `pad10*1` rule, then hash it.
    fn hash_pad(input: &[GenericField<F>]) -> Self::Hash;

    /// Hash the slice if necessary to reduce its length to ~256 bits. If it already fits, this is a
    /// no-op.
    fn hash_or_noop(inputs: &[GenericField<F>]) -> Self::Hash;

    /// absorb the input into the given state
    fn sponge(state: &mut Self::Permutation, input: Vec<GenericField<F>>);

    /// 2-to-1 compression
    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash;

    /// squeeze out a vec of Goldilocks field elements (used for duplex/challenger)
    fn squeeze_goldilocks(state: &mut Self::Permutation) -> Vec<F>;
}

/// Trait for algebraic hash functions, built from a permutation using the sponge construction.
pub trait AlgebraicHasher<F: RichField>: Hasher<F, Hash = HashOut<F>> {
    type AlgebraicPermutation: PlonkyPermutation<Target>;

    /// Circuit to conditionally swap two chunks of the inputs (useful in verifying Merkle proofs),
    /// then apply the permutation.
    fn permute_swapped<const D: usize>(
        inputs: Self::AlgebraicPermutation,
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self::AlgebraicPermutation
        where
            F: RichField + Extendable<D>;
}

/// Generic configuration trait.
pub trait GenericConfig<const D: usize>:
Debug + Clone + Sync + Sized + Send + Eq + PartialEq
{
    /// Main field.
    type F: RichField + Extendable<D, Extension = Self::FE>;

    /// Field extension of degree D of the main field.
    type FE: FieldExtension<D, BaseField = Self::F>;
    /// Hash function used for building Merkle trees.
    type Hasher: Hasher<Self::F>;
    /// Algebraic hash function used for the challenger and hashing public inputs.
    type InnerHasher: AlgebraicHasher<Self::F>;
}

/// Configuration using Poseidon over the Goldilocks field.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Serialize)]
pub struct PoseidonGoldilocksConfig;
impl GenericConfig<2> for PoseidonGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonHash;
    type InnerHasher = PoseidonHash;
}

/// Configuration using truncated Keccak over the Goldilocks field.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct KeccakGoldilocksConfig;
impl GenericConfig<2> for KeccakGoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = KeccakHash<25>;
    type InnerHasher = PoseidonHash;
}

/// Configuration using Poseidon2BN254 as hasher over the Goldilocks field.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct Poseidon2BN254Config;
impl GenericConfig<2> for Poseidon2BN254Config {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = Poseidon2BN254;
    type InnerHasher = PoseidonHash;
}
