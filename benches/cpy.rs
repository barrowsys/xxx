use criterion::{Criterion, criterion_group, criterion_main};
use std::{hint::black_box, ptr};
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
    let mut B = |N, n, x, y: Result<*mut usize, String>| unsafe {
        let y = y.unwrap();
        let N2 = format!("{N} (std)");
        c.bench_function(N, |b| b.iter(|| xxx::cpy(y, x, n)));
        c.bench_function(&N2, |b| b.iter(|| std::ptr::copy(y, x, n)));
    };

    let small = 20;
    let med = 100_000;
    let big = 300_000;
    let huge = 500_000;

    unsafe {
        B("small copy", small, iota(small), xxx::new(small));
        B("med copy", med, iota(med), xxx::new(med));
        B("big copy", big, iota(big), xxx::new(big));
        B("huge copy", huge, iota(huge), xxx::new(huge));
    }
}

criterion_group!(benches, criterion_bench);
criterion_main!(benches);
