#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::fmt;

use anyhow::ensure;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::field::goldilocks_field::GoldilocksField;
use crate::field::types::{Field, PrimeField64, Sample};
use crate::hash::poseidon::Poseidon;
use crate::iop::target::Target;
use crate::plonk::config::{GenericField, GenericHashOut};
use ark_bn254::Fr as BN254Fr;
use crate::hash::poseidon2_bn254::{bytes_to_felts, felts_to_bytes};

/// A prime order field with the features we need to use it as a base field in our argument system.
pub trait RichField: PrimeField64 + Poseidon {}

impl RichField for GoldilocksField {}

/// Hash digest for the BN254 field, contains single Fr element
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BN254HashOut{
    pub element: BN254Fr
}

/// Serialize the bn254 field , uses Arkworks
impl Serialize for BN254HashOut {
    fn serialize < S > ( & self, serializer: S) -> Result < S::Ok, S::Error >
        where S: Serializer {

        let element_to_bytes = felts_to_bytes(&self.element);
        serializer.serialize_bytes( & element_to_bytes)
    }
}
/// Deserialize the bn254 field , uses Arkworks
impl<'de> Deserialize<'de> for BN254HashOut {
    fn deserialize<
        D>(deserializer: D) -> Result< Self,
        D::Error>
        where D: Deserializer<'de> {
        let element_as_bytes = < [u8; 32] >::deserialize(deserializer) ?;
        let mut element_array = < [u8; 32] >::default();
        element_array.copy_from_slice( & element_as_bytes[0..32]);

        let deserialized_element = bytes_to_felts(&element_array);

        Ok( Self {
            element: deserialized_element,
        })
    }
}

/// implement GenericHashOut for the BN254 hash digest
/// `F` here is the goldilocks not the BN254 field
impl<F: RichField> GenericHashOut<F> for BN254HashOut {
    fn to_bytes(&self) -> Vec<u8> {
        felts_to_bytes(&self.element)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 32);
        BN254HashOut{
            element: bytes_to_felts(bytes)
        }
    }

    fn to_vec(&self) -> Vec<GenericField<F>> {
        vec![GenericField::BN254(self.element.clone())]
    }
}

pub const NUM_HASH_OUT_ELTS: usize = 4;

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct HashOut<F: Field> {
    pub elements: [F; NUM_HASH_OUT_ELTS],
}

impl<F: Field> HashOut<F> {
    pub const ZERO: Self = Self {
        elements: [F::ZERO; NUM_HASH_OUT_ELTS],
    };

    // TODO: Switch to a TryFrom impl.
    pub fn from_vec(elements: Vec<F>) -> Self {
        debug_assert!(elements.len() == NUM_HASH_OUT_ELTS);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub fn from_partial(elements_in: &[F]) -> Self {
        let mut elements = [F::ZERO; NUM_HASH_OUT_ELTS];
        elements[0..elements_in.len()].copy_from_slice(elements_in);
        Self { elements }
    }
}

impl<F: Field> From<[F; NUM_HASH_OUT_ELTS]> for HashOut<F> {
    fn from(elements: [F; NUM_HASH_OUT_ELTS]) -> Self {
        Self { elements }
    }
}

impl<F: Field> TryFrom<&[F]> for HashOut<F> {
    type Error = anyhow::Error;

    fn try_from(elements: &[F]) -> Result<Self, Self::Error> {
        ensure!(elements.len() == NUM_HASH_OUT_ELTS);
        Ok(Self {
            elements: elements.try_into().unwrap(),
        })
    }
}

impl<F> Sample for HashOut<F>
where
    F: Field,
{
    #[inline]
    fn sample<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + ?Sized,
    {
        Self {
            elements: [
                F::sample(rng),
                F::sample(rng),
                F::sample(rng),
                F::sample(rng),
            ],
        }
    }
}

impl<F: RichField> GenericHashOut<F> for HashOut<F> {
    fn to_bytes(&self) -> Vec<u8> {
        self.elements
            .into_iter()
            .flat_map(|x| x.to_canonical_u64().to_le_bytes())
            .collect()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        HashOut {
            elements: bytes
                .chunks(8)
                .take(NUM_HASH_OUT_ELTS)
                .map(|x| F::from_canonical_u64(u64::from_le_bytes(x.try_into().unwrap())))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    fn to_vec(&self) -> Vec<GenericField<F>> {
        self.elements
            .iter()
            .copied()
            .map(GenericField::<F>::Goldilocks)
            .collect()
    }
}

impl<F: Field> Default for HashOut<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Represents a ~256 bit hash output.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct HashOutTarget {
    pub elements: [Target; NUM_HASH_OUT_ELTS],
}

impl HashOutTarget {
    // TODO: Switch to a TryFrom impl.
    pub fn from_vec(elements: Vec<Target>) -> Self {
        debug_assert!(elements.len() == NUM_HASH_OUT_ELTS);
        Self {
            elements: elements.try_into().unwrap(),
        }
    }

    pub fn from_partial(elements_in: &[Target], zero: Target) -> Self {
        let mut elements = [zero; NUM_HASH_OUT_ELTS];
        elements[0..elements_in.len()].copy_from_slice(elements_in);
        Self { elements }
    }
}

impl From<[Target; NUM_HASH_OUT_ELTS]> for HashOutTarget {
    fn from(elements: [Target; NUM_HASH_OUT_ELTS]) -> Self {
        Self { elements }
    }
}

impl TryFrom<&[Target]> for HashOutTarget {
    type Error = anyhow::Error;

    fn try_from(elements: &[Target]) -> Result<Self, Self::Error> {
        ensure!(elements.len() == NUM_HASH_OUT_ELTS);
        Ok(Self {
            elements: elements.try_into().unwrap(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleCapTarget(pub Vec<HashOutTarget>);

/// Hash consisting of a byte array.
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct BytesHash<const N: usize>(pub [u8; N]);

impl<const N: usize> Sample for BytesHash<N> {
    #[inline]
    fn sample<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + ?Sized,
    {
        let mut buf = [0; N];
        rng.fill_bytes(&mut buf);
        Self(buf)
    }
}

impl<F: RichField, const N: usize> GenericHashOut<F> for BytesHash<N> {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.try_into().unwrap())
    }

    fn to_vec(&self) -> Vec<GenericField<F>> {
        self.0
            // Chunks of 7 bytes since 8 bytes would allow collisions.
            .chunks(7)
            .map(|bytes| {
                let mut arr = [0u8; 8];
                arr[..bytes.len()].copy_from_slice(bytes);
                let raw = F::from_canonical_u64(u64::from_le_bytes(arr));
                GenericField::<F>::Goldilocks(raw)
            })
            .collect()
    }
}

impl<const N: usize> Serialize for BytesHash<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

struct ByteHashVisitor<const N: usize>;

impl<'de, const N: usize> Visitor<'de> for ByteHashVisitor<N> {
    type Value = BytesHash<N>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array containing exactly {} bytes", N)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut bytes = [0u8; N];
        for i in 0..N {
            let next_element = seq.next_element()?;
            match next_element {
                Some(value) => bytes[i] = value,
                None => return Err(de::Error::invalid_length(i, &self)),
            }
        }
        Ok(BytesHash(bytes))
    }

    fn visit_bytes<E>(self, s: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let bytes = s.try_into().unwrap();
        Ok(BytesHash(bytes))
    }
}

impl<'de, const N: usize> Deserialize<'de> for BytesHash<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ByteHashVisitor::<N>)
    }
}
