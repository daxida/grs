use criterion::{Criterion, black_box, criterion_group, criterion_main};
use grs::tokenizer::tokenize;
use std::fs::File;
use std::io::{self, Read};

fn read_file(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn benchmark_tokenize(c: &mut Criterion) {
    // TODO: This file is too big to include in the repository
    // let file_path = "texts/dump.txt";
    let file_path = "texts/small.txt";
    let content = read_file(file_path).unwrap();

    let mut group = c.benchmark_group("group");
    // group.sample_size(10);

    group.bench_function("tokenize", |b| {
        b.iter(|| tokenize(black_box(&content)));
    });
}

criterion_group!(benches, benchmark_tokenize);
criterion_main!(benches);
