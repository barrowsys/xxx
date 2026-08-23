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
    let big = 100_000;
    let really_big = 300_000;

    unsafe {
        B("small copy", small, iota(small), xxx::new(small));
        B("big copy", big, iota(big), xxx::new(big));
        B(
            "really big copy",
            really_big,
            iota(really_big),
            xxx::new(really_big),
        );
    }
}

criterion_group!(benches, criterion_bench);
criterion_main!(benches);
