use std::{ptr, hint::black_box};
use criterion::{criterion_group, criterion_main, Criterion};
use xxx;

fn iota(x: usize) -> *mut usize {
    unsafe {
        let P: *mut usize = xxx::new(x).unwrap();

        for i in 0..x {
            ptr::write(P.add(i), i);
        }

        P
    }
}

fn criterion_bench(c: &mut Criterion) {
    unsafe {
        let small_n = 20;
        let small_x = iota(small_n);
        let small_y = xxx::new(small_n).unwrap();

        c.bench_function("small copy", |b| b.iter(|| {
            xxx::cpy(small_y, small_x, small_n);
        }));
    }

    unsafe {
        let n = 100_000;
        let x = iota(n);
        let y = xxx::new(n).unwrap();

        c.bench_function("big copy", |b| b.iter(|| {
            xxx::cpy(y, x, n);
        }));
    }

    unsafe {
        let n = 300_000;
        let x = iota(n);
        let y = xxx::new(n).unwrap();

        c.bench_function("really big copy", |b| b.iter(|| {
            xxx::cpy(y, x, n);
        }));
    }
}

criterion_group!(benches, criterion_bench);
criterion_main!(benches);
