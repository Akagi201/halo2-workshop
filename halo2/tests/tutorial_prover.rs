use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Cell, Chip, Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Assigned, Circuit, Column, ConstraintSystem, Error, Fixed, Instance},
    poly::Rotation,
};

#[allow(non_snake_case, dead_code)]
#[derive(Debug, Clone)]
struct TutorialConfig {
    l: Column<Advice>,
    r: Column<Advice>,
    o: Column<Advice>,

    sl: Column<Fixed>,
    sr: Column<Fixed>,
    so: Column<Fixed>,
    sm: Column<Fixed>,
    sc: Column<Fixed>,
    PI: Column<Instance>,
}

struct TutorialChip<F: FieldExt> {
    config: TutorialConfig,
    marker: PhantomData<F>,
}

impl<F: FieldExt> TutorialChip<F> {
    fn new(config: TutorialConfig) -> Self {
        TutorialChip {
            config,
            marker: PhantomData,
        }
    }
}

impl<F: FieldExt> Chip<F> for TutorialChip<F> {
    type Config = TutorialConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

trait TutorialComposer<F: FieldExt> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;

    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>;

    /// Ensure two wire values are the same, in effect connecting the wires to each other
    fn copy(&self, layouter: &mut impl Layouter<F>, a: Cell, b: Cell) -> Result<(), Error>;

    /// Exposes a number as a public input to the circuit.
    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error>;
}

impl<F: FieldExt> TutorialComposer<F> for TutorialChip<F> {
    fn raw_multiply<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {
        layouter.assign_region(
            || "mul",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.l,
                    0,
                    || {
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || values.unwrap().map(|v| v.2),
                )?;

                region.assign_fixed(|| "m", self.config.sm, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn raw_add<FM>(
        &self,
        layouter: &mut impl Layouter<F>,
        mut f: FM,
    ) -> Result<(Cell, Cell, Cell), Error>
    where
        FM: FnMut() -> Value<(Assigned<F>, Assigned<F>, Assigned<F>)>,
    {
        layouter.assign_region(
            || "add",
            |mut region| {
                let mut values = None;
                let lhs = region.assign_advice(
                    || "lhs",
                    self.config.l,
                    0,
                    || {
                        values = Some(f());
                        values.unwrap().map(|v| v.0)
                    },
                )?;
                let rhs = region.assign_advice(
                    || "rhs",
                    self.config.r,
                    0,
                    || values.unwrap().map(|v| v.1),
                )?;

                let out = region.assign_advice(
                    || "out",
                    self.config.o,
                    0,
                    || values.unwrap().map(|v| v.2),
                )?;

                region.assign_fixed(|| "l", self.config.sl, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "r", self.config.sr, 0, || Value::known(F::one()))?;
                region.assign_fixed(|| "o", self.config.so, 0, || Value::known(F::one()))?;

                Ok((lhs.cell(), rhs.cell(), out.cell()))
            },
        )
    }

    fn copy(&self, layouter: &mut impl Layouter<F>, left: Cell, right: Cell) -> Result<(), Error> {
        layouter.assign_region(
            || "copy",
            |mut region| {
                region.constrain_equal(left, right)?;
                region.constrain_equal(left, right)
            },
        )
    }

    fn expose_public(
        &self,
        layouter: &mut impl Layouter<F>,
        cell: Cell,
        row: usize,
    ) -> Result<(), Error> {
        layouter.constrain_instance(cell, self.config.PI, row)
    }
}

#[derive(Default)]
struct TutorialCircuit<F: FieldExt> {
    x: Value<F>,
    y: Value<F>,
    constant: F,
}

impl<F: FieldExt> Circuit<F> for TutorialCircuit<F> {
    type Config = TutorialConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let l = meta.advice_column();
        let r = meta.advice_column();
        let o = meta.advice_column();

        meta.enable_equality(l);
        meta.enable_equality(r);
        meta.enable_equality(o);

        let sm = meta.fixed_column();
        let sl = meta.fixed_column();
        let sr = meta.fixed_column();
        let so = meta.fixed_column();
        let sc = meta.fixed_column();
        #[allow(non_snake_case)]
        let PI = meta.instance_column();
        meta.enable_equality(PI);

        meta.create_gate("mini plonk", |meta| {
            let l = meta.query_advice(l, Rotation::cur());
            let r = meta.query_advice(r, Rotation::cur());
            let o = meta.query_advice(o, Rotation::cur());

            let sl = meta.query_fixed(sl, Rotation::cur());
            let sr = meta.query_fixed(sr, Rotation::cur());
            let so = meta.query_fixed(so, Rotation::cur());
            let sm = meta.query_fixed(sm, Rotation::cur());
            let sc = meta.query_fixed(sc, Rotation::cur());

            vec![l.clone() * sl + r.clone() * sr + l * r * sm + (o * so * (-F::one())) + sc]
        });

        TutorialConfig {
            l,
            r,
            o,
            sl,
            sr,
            so,
            sm,
            sc,
            PI,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let cs = TutorialChip::new(config);

        // Initialise these values so that we can access them more easily outside the block we actually give them a value in
        let x: Value<Assigned<_>> = self.x.into();
        let y: Value<Assigned<_>> = self.y.into();
        let consty = Assigned::from(self.constant);

        // Create x squared
        // Note that the variables named ai for some i are just place holders, meaning that a0 isn't
        // necessarily the first entry in the column a; though in the code we try to make things clear
        let (a0, b0, c0) = cs.raw_multiply(&mut layouter, || x.map(|x| (x, x, x * x)))?;
        cs.copy(&mut layouter, a0, b0)?;

        // Create y squared
        let (a1, b1, c1) = cs.raw_multiply(&mut layouter, || y.map(|y| (y, y, y * y)))?;
        cs.copy(&mut layouter, a1, b1)?;

        // Create xy squared
        let (a2, b2, c2) = cs.raw_multiply(&mut layouter, || {
            x.zip(y).map(|(x, y)| (x * x, y * y, x * x * y * y))
        })?;
        cs.copy(&mut layouter, c0, a2)?;
        cs.copy(&mut layouter, c1, b2)?;

        // Add the constant
        let (a3, b3, c3) = cs.raw_add(&mut layouter, || {
            x.zip(y)
                .map(|(x, y)| (x * x * y * y, consty, x * x * y * y + consty))
        })?;
        cs.copy(&mut layouter, c2, a3)?;

        // Ensure that the constant in the TutorialCircuit struct is correctly used and that the
        // result of the circuit computation is what is expected. (use expose_public))
        cs.expose_public(&mut layouter, b3, 0)?;
        // Below is another way to expose a public value, this time the output value of the computation
        // (Use constrain_instance)
        layouter.constrain_instance(c3, cs.config.PI, 1)?;

        Ok(())
    }
}

#[test]
fn tutorial_prover() {
    use halo2_proofs::{
        circuit::Value,
        halo2curves::bn256::{Bn256, Fr as Fp, G1Affine},
        plonk::*,
        poly::{
            commitment::ParamsProver,
            kzg::{
                commitment::{KZGCommitmentScheme, ParamsKZG},
                multiopen::{ProverGWC, VerifierGWC},
                strategy::SingleStrategy,
            },
        },
        transcript::{
            Blake2bRead, Blake2bWrite, Challenge255, TranscriptReadBuffer, TranscriptWriterBuffer,
        },
    };
    use rand_core::OsRng;

    // The number of rows in our circuit cannot exceed 2^k. Since our example
    // circuit is very small, we can pick a very small value here.
    let k = 4;

    let constant = Fp::from(7);
    let x = Fp::from(5);
    let y = Fp::from(9);
    let z = Fp::from(25 * 81 + 7);

    // Create an empty circuit to make the verifying key from (we could use a complete circuit too)
    let empty_circuit: TutorialCircuit<Fp> = TutorialCircuit {
        x: Value::unknown(),
        y: Value::unknown(),
        constant,
    };
    // Create the parameters we need to make the proof (under the hood mathematics)
    let params: ParamsKZG<Bn256> = ParamsKZG::new(k);
    let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

    // Create a circuit with the starting input provided
    let circuit: TutorialCircuit<Fp> = TutorialCircuit {
        x: Value::known(x),
        y: Value::known(y),
        constant,
    };

    // Initialise the transcript, where things needed for the proof will be stored
    let mut transcript: Blake2bWrite<Vec<u8>, G1Affine, Challenge255<G1Affine>> =
        Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
    create_proof::<KZGCommitmentScheme<Bn256>, ProverGWC<Bn256>, _, _, _, _>(
        &params,
        &pk,
        &[circuit],
        &[&[&[constant, z]]],
        OsRng,
        &mut transcript,
    )
    .expect("proof generation should not fail");
    // Derive the actual proof from the transcript data
    let proof = transcript.finalize();

    // Notice that this transcript is to be read from, not written to
    let transcript = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
    let strategy = SingleStrategy::new(&params);
    verify_proof::<_, VerifierGWC<Bn256>, _, _, _>(
        &params,
        pk.get_vk(),
        strategy.clone(),
        &[&[&[constant, z]]],
        &mut transcript.clone(),
    )
    .unwrap();
}