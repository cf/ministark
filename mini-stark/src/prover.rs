use crate::challenges::Challenges;
use crate::channel::ProverChannel;
use crate::constraint_evaluator::ConstraintEvaluator;
use crate::merkle::MerkleTree;
use crate::utils::Timer;
use crate::Air;
use crate::Constraint;
use crate::Matrix;
use crate::Trace;
use crate::TraceInfo;
use ark_ff::One;
use ark_ff::UniformRand;
use ark_ff::Zero;
use ark_poly::domain::Radix2EvaluationDomain;
use ark_poly::univariate::DensePolynomial;
use ark_poly::DenseUVPolynomial;
use ark_poly::EvaluationDomain;
use ark_poly::Polynomial;
use ark_serialize::CanonicalDeserialize;
use ark_serialize::CanonicalSerialize;
use fast_poly::allocator::PageAlignedAllocator;
use fast_poly::plan::PLANNER;
use fast_poly::stage::MulPowStage;
use fast_poly::utils::buffer_no_copy;
use fast_poly::GpuField;
use sha2::Sha256;
use std::time::Instant;

// TODO: include ability to specify:
// - base field
// - extension field
// - hashing function
// - determine if grinding factor is appropriate
// - fri folding factor
// - fri max remainder size
#[derive(Debug, Clone, Copy, CanonicalSerialize, CanonicalDeserialize)]
pub struct ProofOptions {
    pub num_queries: u8,
    // would be nice to make this clear as LDE blowup factor vs constraint blowup factor
    pub blowup_factor: u8,
}

impl ProofOptions {
    pub fn new(num_queries: u8, blowup_factor: u8) -> Self {
        ProofOptions {
            num_queries,
            blowup_factor,
        }
    }
}

/// A proof generated by a mini-stark prover
#[derive(Debug, Clone)]
pub struct Proof {
    options: ProofOptions,
    trace_info: TraceInfo,
    commitments: Vec<u64>,
}

/// Errors that can occur during the proving stage
#[derive(Debug)]
pub enum ProvingError {
    Fail,
    // /// This error occurs when a transition constraint evaluated over a specific execution
    // trace /// does not evaluate to zero at any of the steps.
    // UnsatisfiedTransitionConstraintError(usize),
    // /// This error occurs when polynomials built from the columns of a constraint evaluation
    // /// table do not all have the same degree.
    // MismatchedConstraintPolynomialDegree(usize, usize),
}

pub trait Prover {
    type Fp: GpuField;
    type Air: Air<Fp = Self::Fp>;
    type Trace: Trace<Fp = Self::Fp>;

    fn new(options: ProofOptions) -> Self;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> <Self::Air as Air>::PublicInputs;

    fn options(&self) -> ProofOptions;

    /// Return value is of the form `(lde, polys, merkle_tree)`
    fn build_trace_commitment(
        &self,
        trace: &Matrix<Self::Fp>,
        trace_domain: Radix2EvaluationDomain<Self::Fp>,
        lde_domain: Radix2EvaluationDomain<Self::Fp>,
    ) -> (Matrix<Self::Fp>, Matrix<Self::Fp>, MerkleTree<Sha256>) {
        let trace_polys = {
            let _timer = Timer::new("trace interpolation");
            trace.interpolate_columns(trace_domain)
        };
        let trace_lde = {
            let _timer = Timer::new("trace low degree extension");
            trace_polys.evaluate(lde_domain)
        };
        let merkle_tree = {
            let _timer = Timer::new("trace commitment");
            trace_lde.commit_to_rows()
        };
        (trace_lde, trace_polys, merkle_tree)
    }

    /// builds a commitment to the combined constraint quotient evaluations.
    /// Output is of the form `(lde, poly, lde_merkle_tree)`
    fn build_constraint_commitment(
        &self,
        composed_evaluations: Matrix<Self::Fp>,
        composition_poly: Matrix<Self::Fp>,
        air: &Self::Air,
    ) -> (Matrix<Self::Fp>, Matrix<Self::Fp>, MerkleTree<Sha256>) {
        let num_composed_columns = composition_poly.num_rows() / air.trace_len();
        let transposed_evals = Matrix::from_rows(
            composed_evaluations.0[0]
                .chunks(num_composed_columns)
                .map(|chunk| chunk.to_vec())
                .collect(),
        );
        let transposed_poly = Matrix::from_rows(
            composition_poly.0[0]
                .chunks(num_composed_columns)
                .map(|chunk| chunk.to_vec())
                .collect(),
        );
        let merkle_tree = transposed_evals.commit_to_rows();
        (transposed_evals, transposed_poly, merkle_tree)
    }

    fn generate_proof(&self, trace: Self::Trace) -> Result<Proof, ProvingError> {
        let _timer = Timer::new("proof generation");

        let options = self.options();
        let trace_info = trace.info();
        let pub_inputs = self.get_pub_inputs(&trace);
        let air = Self::Air::new(trace_info.clone(), pub_inputs, options);
        let mut channel = ProverChannel::<Self::Air, Sha256>::new(&air);

        {
            let ce_blowup_factor = air.ce_blowup_factor();
            let lde_blowup_factor = air.lde_blowup_factor();
            assert!(ce_blowup_factor <= lde_blowup_factor, "constraint evaluation blowup factor {ce_blowup_factor} is larger than the lde blowup factor {lde_blowup_factor}");
        }

        let trace_domain = air.trace_domain();
        let lde_domain = air.lde_domain();
        let (base_trace_lde, base_trace_polys, base_trace_lde_tree) =
            self.build_trace_commitment(trace.base_columns(), trace_domain, lde_domain);

        channel.commit_base_trace(base_trace_lde_tree.root());
        // let num_challenges = 20;
        // TODO:
        let num_challenges = air.num_challenges();
        println!("NUM CHALLENGE: {num_challenges}");
        let challenges = channel.get_challenges::<Self::Fp>(num_challenges);

        let mut trace_lde = base_trace_lde;
        let mut trace_polys = base_trace_polys;
        let mut extension_trace_tree = None;

        if let Some(extension_matrix) = trace.build_extension_columns(&challenges) {
            let (extension_trace_lde, extension_trace_polys, extension_trace_lde_tree) =
                self.build_trace_commitment(&extension_matrix, trace_domain, lde_domain);
            channel.commit_extension_trace(extension_trace_lde_tree.root());
            // TODO: this approach could be better
            extension_trace_tree = Some(extension_trace_lde_tree);
            trace_lde.append(extension_trace_lde);
            trace_polys.append(extension_trace_polys);
        }

        // TODO: don't re-evaluate. Just keep matrix of trace values
        #[cfg(debug_assertions)]
        air.validate(&challenges, &trace_polys.evaluate(trace_domain));

        let challenge_coeffs = channel.get_constraint_composition_coeffs();
        let constraint_evaluator = ConstraintEvaluator::new(&air, challenge_coeffs);
        let composed_evaluations = constraint_evaluator.evaluate(&challenges, &trace_lde);
        let mut composition_poly = composed_evaluations.interpolate_columns(lde_domain);

        // TODO: Clean up
        let composition_degree = composition_poly.column_degrees()[0];
        assert_eq!(composition_degree, air.composition_degree());
        composition_poly.0[0].truncate(composition_degree + 1);

        let (composition_trace_lde, composition_trace_poly, composition_trace_lde_tree) =
            self.build_constraint_commitment(composed_evaluations, composition_poly, &air);

        // let z = channel.get_ood_point();

        Ok(Proof {
            options,
            trace_info,
            commitments: Vec::new(),
        })
    }
}
