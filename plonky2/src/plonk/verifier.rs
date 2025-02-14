//! plonky2 verifier implementation.

use anyhow::{ensure, Result};

use crate::field::extension::Extendable;
use crate::field::types::Field;
use crate::fri::verifier::verify_fri_proof;
use crate::hash::hash_types::RichField;
use crate::hash::hashing::*;
use crate::plonk::circuit_data::{CommonCircuitData, VerifierOnlyCircuitData};
use crate::plonk::config::{GenericConfig, Hasher};
use crate::plonk::plonk_common::reduce_with_powers;
use crate::plonk::proof::{Proof, ProofChallenges, ProofWithPublicInputs};
use crate::plonk::validate_shape::validate_proof_with_pis_shape;
use crate::plonk::vanishing_poly::eval_vanishing_poly;
use crate::plonk::vars::EvaluationVars;

// debugging features in the verifier
#[derive(Debug,Clone,PartialEq,PartialOrd)]
pub enum HashStatisticsPrintLevel {
    None,
    Summary,
    Info,
    Debug,
}

#[derive(Debug,Clone)]
pub struct VerifierOptions {
    pub print_hash_statistics: HashStatisticsPrintLevel,
}

pub const DEFAULT_VERIFIER_OPTIONS: VerifierOptions = VerifierOptions {
    print_hash_statistics: HashStatisticsPrintLevel::None,
};

pub(crate) fn verify<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof_with_pis: ProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
) -> Result<()> {
    verify_with_options(
        proof_with_pis,
        verifier_data,
        common_data,
        &DEFAULT_VERIFIER_OPTIONS,
    )
}
pub(crate) fn verify_with_options<F: RichField + Extendable<D>, C: GenericConfig<D, F = F>, const D: usize>(
    proof_with_pis: ProofWithPublicInputs<F, C, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
    verifier_options: &VerifierOptions,
) -> Result<()> {

    reset_hash_counters();

    validate_proof_with_pis_shape(&proof_with_pis, common_data)?;

    let public_inputs_hash = proof_with_pis.get_public_inputs_hash();

    if verifier_options.print_hash_statistics >= HashStatisticsPrintLevel::Info {
        print_hash_counters("after PI");
    }

    let challenges = proof_with_pis.get_challenges(
        public_inputs_hash,
        &verifier_data.circuit_digest,
        common_data,
    )?;

    if verifier_options.print_hash_statistics >= HashStatisticsPrintLevel::Info {
        print_hash_counters("after challenges");
    }

    let result = verify_with_challenges::<F, C, D>(
        proof_with_pis.proof,
        public_inputs_hash,
        challenges,
        verifier_data,
        common_data,
        verifier_options
    );

    if verifier_options.print_hash_statistics >= HashStatisticsPrintLevel::Summary {
        print_hash_counters("verify total");
    }
    result
}

pub(crate) fn verify_with_challenges<
    F: RichField + Extendable<D>,
    C: GenericConfig<D, F = F>,
    const D: usize,
>(
    proof: Proof<F, C, D>,
    public_inputs_hash: <<C as GenericConfig<D>>::InnerHasher as Hasher<F>>::Hash,
    challenges: ProofChallenges<F, D>,
    verifier_data: &VerifierOnlyCircuitData<C, D>,
    common_data: &CommonCircuitData<F, D>,
    verifier_options: &VerifierOptions,
) -> Result<()> {
    let local_constants = &proof.openings.constants;
    let local_wires = &proof.openings.wires;
    let vars = EvaluationVars {
        local_constants,
        local_wires,
        public_inputs_hash: &public_inputs_hash,
    };
    let local_zs = &proof.openings.plonk_zs;
    let next_zs = &proof.openings.plonk_zs_next;
    let local_lookup_zs = &proof.openings.lookup_zs;
    let next_lookup_zs = &proof.openings.lookup_zs_next;
    let s_sigmas = &proof.openings.plonk_sigmas;
    let partial_products = &proof.openings.partial_products;

    // Evaluate the vanishing polynomial at our challenge point, zeta.
    let vanishing_polys_zeta = eval_vanishing_poly::<F, D>(
        common_data,
        challenges.plonk_zeta,
        vars,
        local_zs,
        next_zs,
        local_lookup_zs,
        next_lookup_zs,
        partial_products,
        s_sigmas,
        &challenges.plonk_betas,
        &challenges.plonk_gammas,
        &challenges.plonk_alphas,
        &challenges.plonk_deltas,
    );

    // Check each polynomial identity, of the form `vanishing(x) = Z_H(x) quotient(x)`, at zeta.
    let quotient_polys_zeta = &proof.openings.quotient_polys;
    let zeta_pow_deg = challenges
        .plonk_zeta
        .exp_power_of_2(common_data.degree_bits());
    let z_h_zeta = zeta_pow_deg - F::Extension::ONE;
    // `quotient_polys_zeta` holds `num_challenges * quotient_degree_factor` evaluations.
    // Each chunk of `quotient_degree_factor` holds the evaluations of `t_0(zeta),...,t_{quotient_degree_factor-1}(zeta)`
    // where the "real" quotient polynomial is `t(X) = t_0(X) + t_1(X)*X^n + t_2(X)*X^{2n} + ...`.
    // So to reconstruct `t(zeta)` we can compute `reduce_with_powers(chunk, zeta^n)` for each
    // `quotient_degree_factor`-sized chunk of the original evaluations.
    for (i, chunk) in quotient_polys_zeta
        .chunks(common_data.quotient_degree_factor)
        .enumerate()
    {
        ensure!(vanishing_polys_zeta[i] == z_h_zeta * reduce_with_powers(chunk, zeta_pow_deg));
    }

    let merkle_caps = &[
        verifier_data.constants_sigmas_cap.clone(),
        proof.wires_cap,
        // In the lookup case, `plonk_zs_partial_products_cap` should also include the lookup commitment.
        proof.plonk_zs_partial_products_cap,
        proof.quotient_polys_cap,
    ];

    verify_fri_proof::<F, C, D>(
        &common_data.get_fri_instance(challenges.plonk_zeta),
        &proof.openings.to_fri_openings(),
        &challenges.fri_challenges,
        merkle_caps,
        &proof.opening_proof,
        &common_data.fri_params,
        &verifier_options,
    )?;

    Ok(())
}
