use criterion::measurement::{Measurement, ValueFormatter, WallTime};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use learncoin_lib::{
    BlockHash, MerkleTree, ProofOfWork, Sha256, Transaction, TransactionInput, TransactionOutput,
};

struct HashPowerFormatter {}

impl ValueFormatter for HashPowerFormatter {
    fn scale_values(&self, _typical_value: f64, _values: &mut [f64]) -> &'static str {
        "H/s"
    }

    fn scale_throughputs(
        &self,
        _typical_value: f64,
        _throughput: &Throughput,
        _values: &mut [f64],
    ) -> &'static str {
        "H/s"
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "H/s"
    }
}

struct HashPower {}

impl Measurement for HashPower {
    type Intermediate = u64;
    type Value = u64;

    fn start(&self) -> Self::Intermediate {
        0
    }

    fn end(&self, i: Self::Intermediate) -> Self::Value {
        i
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        *v1 + *v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &HashPowerFormatter {}
    }
}

fn create_transactions() -> Vec<Transaction> {
    let amount = 50;
    let inputs = vec![TransactionInput::new_coinbase()];
    let outputs = vec![TransactionOutput::new(amount)];
    vec![Transaction::new(inputs, outputs).unwrap()]
}

fn compute_nonce_benchmark(c: &mut Criterion<WallTime>) {
    // Figure out how many nonce values are tested for the given block header.
    // Then use the nonce value to measure throughput for each iteration, which gives us
    // a rough idea on how many hashes our function can test every second.
    // Note that the performance is not the goal, but knowing what's happening is always nice.
    const DIFFICULTY: u32 = 16;
    const TIMESTAMP: u64 = 123456;
    let previous_block_hash = BlockHash::new(Sha256::from_raw([0; 32]));
    let merkle_root = MerkleTree::merkle_root_from_transactions(&create_transactions());
    let nonce =
        ProofOfWork::compute_nonce(&previous_block_hash, &merkle_root, TIMESTAMP, DIFFICULTY);
    let nonce = nonce.unwrap();

    let mut group = c.benchmark_group("Proof of Work");
    group.throughput(Throughput::Elements(nonce as u64));

    // Now we run the actual benchmark.
    group.bench_function("compute_nonce for difficulty 16", |b| {
        b.iter(|| {
            let nonce = ProofOfWork::compute_nonce(
                &previous_block_hash,
                &merkle_root,
                black_box(TIMESTAMP),
                black_box(DIFFICULTY),
            );
            black_box(nonce);
        })
    });
    group.finish();
}

criterion_group!(benches, compute_nonce_benchmark);

criterion_main!(benches);
