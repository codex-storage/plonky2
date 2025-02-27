use plonky2_maybe_rayon::ParallelIterator;
use plonky2_maybe_rayon::rayon::iter::IntoParallelIterator;
use crate::hash::hash_types::RichField;
use crate::hash::hashing::PlonkyPermutation;
use crate::plonk::config::{GenericField, Hasher, HasherField};
use plonky2_field::types::PrimeField64;

#[derive(Debug, Clone)]
pub enum DuplexState<F: RichField, H: Hasher<F>> {
    Absorbing {
        state: H::Permutation,
        buf: Vec<GenericField<F>>, // Buffer for absorbing inputs.
    },
    Squeezing {
        state: H::Permutation,
        buf: Vec<F>, // Buffer holding squeezed outputs.
    },
}

impl<F: RichField, H: Hasher<F>> DuplexState<F,H> {
    /// creates a new duplex state in absorbing mode with an initial zero state.
    pub fn new() -> Self {
        DuplexState::Absorbing {
            state: H::Permutation::new(core::iter::repeat(H::HF::get_zero())),
            buf: Vec::new(),
        }
    }

    /// absorb a generic field element.
    /// In absorbing mode: the element is appended to the buffer.
    /// In squeezing mode: we trash any current outputs and switch back to absorbing.
    pub fn absorb(&mut self, element: GenericField<F>) {
        match self {
            DuplexState::Absorbing { buf, .. } => {
                buf.push(element);
            }
            DuplexState::Squeezing { state, .. } => {
                let mut buf = Vec::new();
                buf.push(element);
                *self = DuplexState::Absorbing {
                    state: state.clone(),
                    buf,
                };
            }
        }
    }

    /// Squeeze out a single challenge element (Goldilocks field elements)
    /// In absorbing mode: the buffer elements are absorbed by calling the sponge
    /// and switching the state to `Squeezing` and filling the output buffer with Goldilocks elems
    /// In squeezing mode: we take elements from the buffer, if buffer is empty we permute and fill the buffer.
    pub fn squeeze(&mut self) -> F {
        match self {
            DuplexState::Absorbing { state, buf, .. } => {
                let input: Vec<GenericField<F>> = buf.drain(..).collect();
                H::sponge(state, input);
                let out_buf: Vec<F> = Self::squeeze_f(state);
                // switch
                *self = DuplexState::Squeezing {
                    state: state.clone(),
                    buf: out_buf,
                };
                // fall back to squeezing.
                self.squeeze()
            }
            DuplexState::Squeezing { state, buf, .. } => {
                if buf.is_empty() {
                    // If the buffer is empty, permute to refill it.
                    state.permute();
                    *buf = Self::squeeze_f(state);
                }
                let e = buf.pop().expect("Output buffer should not be empty");
                e
            }
        }
    }

    /// squeeze out goldilocks field elements from the state
    fn squeeze_f(state: &mut H::Permutation) -> Vec<F>{
        let out = H::squeeze_goldilocks(state);
        assert!(out.len()>0);
        out
    }

    /// grind moved here from the FRI prover
    /// it handles both modes (`Absorbing` and `Squeezing`)
    pub fn grind(&mut self, min_leading_zeros: u32) -> F {
        match self {
            DuplexState::Absorbing { state, buf, .. } => {

                let duplex_intermediate_state = state.clone();
                let buf_felts: Vec<GenericField<F>> = buf.clone();

                Self::grind_helper(duplex_intermediate_state, buf_felts, min_leading_zeros)
            }
            DuplexState::Squeezing { state, .. } => {
                let duplex_intermediate_state = state.clone();
                let buf_felts = vec![];
                Self::grind_helper(duplex_intermediate_state, buf_felts, min_leading_zeros)
            }
        }
    }

    fn grind_helper(state: H::Permutation, input: Vec<GenericField<F>>, min_leading_zeros: u32) -> F {
        let pow_witness = (0..=F::NEG_ONE.to_canonical_u64())
            .into_par_iter()
            .find_any(|&candidate| {
                let mut duplex_state = state.clone();
                let mut sponge_input = input.clone();
                sponge_input.push(GenericField::Goldilocks(F::from_canonical_u64(candidate)));
                H::sponge(&mut duplex_state, sponge_input);
                let temp_buf = Self::squeeze_f(&mut duplex_state);
                let pow_response = temp_buf.iter().last().unwrap();
                let leading_zeros = PrimeField64::to_canonical_u64(pow_response).leading_zeros();
                leading_zeros >= min_leading_zeros
            })
            .map(F::from_canonical_u64)
            .expect("Proof of work failed. This is highly unlikely!");
        pow_witness
    }
}