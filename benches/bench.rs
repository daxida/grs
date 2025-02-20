use criterion::{black_box, criterion_group, criterion_main, Criterion};
use grac::{is_greek_word, split_word_punctuation};
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

    // Word level benches
    let words: Vec<_> = content
        .split_inclusive(|c: char| c.is_whitespace())
        .collect();
    group.bench_function("split_word_punctuation", |b| {
        b.iter(|| {
            for &w in &words {
                let _ = split_word_punctuation(black_box(w));
            }
        });
    });
    group.bench_function("is_greek_word", |b| {
        b.iter(|| {
            for &w in &words {
                let _ = is_greek_word(black_box(w));
            }
        });
    });

    // References
    group.bench_function("ref_split_whitespace_collect", |b| {
        b.iter(|| {
            let _: Vec<_> = black_box(content.split_whitespace().collect());
        });
    });
    group.bench_function("ref_split_whitespace_push", |b| {
        b.iter(|| {
            let mut words = Vec::new();
            for word in black_box(content.split_whitespace()) {
                words.push(word);
            }
            black_box(words);
        });
    });
}

criterion_group!(benches, benchmark_tokenize);
criterion_main!(benches);
