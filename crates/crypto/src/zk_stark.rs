use std::fmt;

use thiserror::Error;
use winterfell::{
    crypto::{hashers::Blake3_256, DefaultRandomCoin, MerkleTree},
    math::{fields::f64::BaseElement, FieldElement},
    matrix::ColMatrix,
    AcceptableOptions, Air, AirContext, Assertion, AuxRandElements, BatchingMethod,
    CompositionPoly, CompositionPolyTrace, ConstraintCompositionCoefficients,
    DefaultConstraintCommitment, DefaultConstraintEvaluator, DefaultTraceLde, EvaluationFrame,
    FieldExtension, PartitionOptions, Proof, ProofOptions, Prover, ProverError, StarkDomain, Trace,
    TraceInfo, TracePolyTable, TraceTable, TransitionConstraintDegree, VerifierError,
};

const TRACE_WIDTH: usize = 2;

/// Convenience aliases for the hash function and Fiat-Shamir coin used in the protocol.
type Blake3 = Blake3_256<BaseElement>;
type RandomCoin = DefaultRandomCoin<Blake3>;
type CommitmentScheme = MerkleTree<Blake3>;

/// Strongly typed wrapper around a Winterfell proof describing execution of a Fibonacci trace.
#[derive(Clone)]
pub struct StarkProof {
    sequence_length: usize,
    result: BaseElement,
    proof: Proof,
}

impl StarkProof {
    /// Returns the number of terms produced by the underlying Fibonacci computation.
    pub fn sequence_length(&self) -> usize {
        self.sequence_length
    }

    /// Returns the final term of the Fibonacci sequence as an integer.
    pub fn result(&self) -> u64 {
        self.result.as_int()
    }

    /// Returns the raw Winterfell proof object.
    pub fn inner(&self) -> &Proof {
        &self.proof
    }

    /// Serializes the proof into bytes suitable for storage or transport.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.proof.to_bytes()
    }

    /// Reconstruct a `StarkProof` instance from serialized bytes and public inputs.
    pub fn from_bytes(
        sequence_length: usize,
        result: u64,
        bytes: &[u8],
    ) -> Result<Self, StarkProofError> {
        if sequence_length < 4 || !sequence_length.is_power_of_two() {
            return Err(StarkProofError::InvalidSequenceLength);
        }

        let proof = Proof::from_bytes(bytes)
            .map_err(|err| StarkProofError::Deserialization(err.to_string()))?;
        Ok(Self {
            sequence_length,
            result: BaseElement::new(result),
            proof,
        })
    }
}

impl fmt::Debug for StarkProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StarkProof")
            .field("sequence_length", &self.sequence_length)
            .field("result", &self.result.as_int())
            .finish_non_exhaustive()
    }
}

/// Errors that can be encountered while generating or verifying STARK proofs.
#[derive(Debug, Error)]
pub enum StarkProofError {
    /// The caller supplied an invalid sequence length.
    #[error("sequence length must be a power of two and at least 4 steps long")]
    InvalidSequenceLength,
    /// Proof generation failed inside the Winterfell prover.
    #[error("failed to generate STARK proof: {0}")]
    Proving(#[from] ProverError),
    /// Verification failed for the supplied proof.
    #[error("failed to verify STARK proof: {0}")]
    Verification(#[from] VerifierError),
    /// Deserialization of proof bytes failed.
    #[error("failed to decode STARK proof: {0}")]
    Deserialization(String),
}

/// Generate a STARK proof attesting that the Fibonacci sequence was computed correctly up to the
/// requested number of terms.
pub fn generate_fibonacci_proof(sequence_length: usize) -> Result<StarkProof, StarkProofError> {
    if sequence_length < 4 || !sequence_length.is_power_of_two() {
        return Err(StarkProofError::InvalidSequenceLength);
    }

    let options = default_proof_options();
    let prover = FibonacciProver::new(options.clone());

    let trace = prover.build_trace(sequence_length);
    let last_step = trace.length() - 1;
    let result = trace.get(1, last_step);
    let proof = prover.prove(trace)?;

    Ok(StarkProof {
        sequence_length,
        result,
        proof,
    })
}

/// Verify a previously generated Fibonacci STARK proof.
pub fn verify_fibonacci_proof(proof: &StarkProof) -> Result<(), StarkProofError> {
    // Ensure the verifier accepts the exact proof options that were used by the prover.
    let acceptable = AcceptableOptions::OptionSet(vec![proof.proof.options().clone()]);

    winterfell::verify::<FibonacciAir, Blake3, RandomCoin, CommitmentScheme>(
        proof.proof.clone(),
        proof.result,
        &acceptable,
    )
    .map_err(StarkProofError::from)
}

fn default_proof_options() -> ProofOptions {
    ProofOptions::new(
        28, // queries
        4,  // blowup factor
        16, // grinding factor
        FieldExtension::None,
        4,  // FRI folding factor
        31, // FRI remainder max degree
        BatchingMethod::Linear,
        BatchingMethod::Linear,
    )
}

// AIR IMPLEMENTATION
// ================================================================================================

struct FibonacciAir {
    context: AirContext<BaseElement>,
    result: BaseElement,
}

impl Air for FibonacciAir {
    type BaseField = BaseElement;
    type PublicInputs = BaseElement;

    fn new(trace_info: TraceInfo, pub_inputs: Self::BaseField, options: ProofOptions) -> Self {
        let degrees = vec![
            TransitionConstraintDegree::new(1),
            TransitionConstraintDegree::new(1),
        ];
        assert_eq!(TRACE_WIDTH, trace_info.width());

        FibonacciAir {
            context: AirContext::new(trace_info, degrees, 3, options),
            result: pub_inputs,
        }
    }

    fn context(&self) -> &AirContext<Self::BaseField> {
        &self.context
    }

    fn evaluate_transition<E: FieldElement + From<Self::BaseField>>(
        &self,
        frame: &EvaluationFrame<E>,
        _periodic_values: &[E],
        result: &mut [E],
    ) {
        let current = frame.current();
        let next = frame.next();

        debug_assert_eq!(TRACE_WIDTH, current.len());
        debug_assert_eq!(TRACE_WIDTH, next.len());

        // Enforce Fibonacci transitions: each row advances the sequence by two terms.
        result[0] = next[0] - (current[0] + current[1]);
        result[1] = next[1] - (current[1] + next[0]);
    }

    fn get_assertions(&self) -> Vec<Assertion<Self::BaseField>> {
        let last_step = self.trace_length() - 1;
        vec![
            Assertion::single(0, 0, Self::BaseField::ONE),
            Assertion::single(1, 0, Self::BaseField::ONE),
            Assertion::single(1, last_step, self.result),
        ]
    }
}

// PROVER IMPLEMENTATION
// ================================================================================================

struct FibonacciProver {
    options: ProofOptions,
}

impl FibonacciProver {
    fn new(options: ProofOptions) -> Self {
        Self { options }
    }

    fn build_trace(&self, sequence_length: usize) -> TraceTable<BaseElement> {
        let steps = sequence_length / 2;
        let mut trace = TraceTable::new(TRACE_WIDTH, steps);

        trace.fill(
            |state| {
                state[0] = BaseElement::ONE;
                state[1] = BaseElement::ONE;
            },
            |_, state| {
                state[0] += state[1];
                state[1] += state[0];
            },
        );

        trace
    }
}

impl Prover for FibonacciProver {
    type BaseField = BaseElement;
    type Air = FibonacciAir;
    type Trace = TraceTable<BaseElement>;
    type HashFn = Blake3;
    type VC = CommitmentScheme;
    type RandomCoin = RandomCoin;
    type TraceLde<E: FieldElement<BaseField = Self::BaseField>> =
        DefaultTraceLde<E, Self::HashFn, Self::VC>;
    type ConstraintCommitment<E: FieldElement<BaseField = Self::BaseField>> =
        DefaultConstraintCommitment<E, Self::HashFn, Self::VC>;
    type ConstraintEvaluator<'a, E: FieldElement<BaseField = Self::BaseField>> =
        DefaultConstraintEvaluator<'a, Self::Air, E>;

    fn get_pub_inputs(&self, trace: &Self::Trace) -> BaseElement {
        let last_step = trace.length() - 1;
        trace.get(1, last_step)
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }

    fn new_trace_lde<E: FieldElement<BaseField = Self::BaseField>>(
        &self,
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<Self::BaseField>,
        domain: &StarkDomain<Self::BaseField>,
        partition_option: PartitionOptions,
    ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
        DefaultTraceLde::new(trace_info, main_trace, domain, partition_option)
    }

    fn new_evaluator<'a, E: FieldElement<BaseField = Self::BaseField>>(
        &self,
        air: &'a Self::Air,
        aux_rand_elements: Option<AuxRandElements<E>>,
        composition_coefficients: ConstraintCompositionCoefficients<E>,
    ) -> Self::ConstraintEvaluator<'a, E> {
        DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
    }

    fn build_constraint_commitment<E: FieldElement<BaseField = Self::BaseField>>(
        &self,
        composition_poly_trace: CompositionPolyTrace<E>,
        num_constraint_composition_columns: usize,
        domain: &StarkDomain<Self::BaseField>,
        partition_options: PartitionOptions,
    ) -> (Self::ConstraintCommitment<E>, CompositionPoly<E>) {
        DefaultConstraintCommitment::new(
            composition_poly_trace,
            num_constraint_composition_columns,
            domain,
            partition_options,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fibonacci_u64(n: usize) -> u64 {
        let mut a: u64 = 1;
        let mut b: u64 = 1;
        for _ in 2..n {
            let next = a + b;
            a = b;
            b = next;
        }
        b
    }

    #[test]
    fn generates_and_verifies_proof() {
        let proof = generate_fibonacci_proof(32).expect("proof generation should succeed");
        assert_eq!(proof.sequence_length(), 32);
        assert_eq!(proof.result(), fibonacci_u64(32));
        verify_fibonacci_proof(&proof).expect("verification should succeed");
    }

    #[test]
    fn detecting_tampering() {
        let mut proof = generate_fibonacci_proof(32).expect("proof generation should succeed");
        proof.result += BaseElement::ONE;
        assert!(verify_fibonacci_proof(&proof).is_err());
    }
}
