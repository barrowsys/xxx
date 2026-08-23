#![feature(likely_unlikely)]
#![feature(repr_simd)]
#![allow(internal_features)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{
    alloc::{self, Layout},
    hint::unlikely,
    mem::{align_of, size_of},
    ptr,
};

pub type R<X> = Result<X, String>;

#[repr(simd)]
#[derive(Copy, Clone)]
struct xmm_t([u8; 16]);

#[repr(simd)]
#[derive(Copy, Clone)]
struct ymm_t([u8; 32]);

#[inline(always)]
pub unsafe fn new<X>(Z: usize) -> R<*mut X> {
    if unlikely(Z == 0) {
        return Err("cannot alloc(0).".to_string());
    }

    /* create an array */
    let lay = Layout::from_size_align(size_of::<X>() * Z, align_of::<X>())
        .map_err(|e| format!("{e}"))?;
    let ptr = unsafe { alloc::alloc(lay) as *mut X };

    /* check for oom */
    if unlikely(ptr as *const X == ptr::null()) {
        return Err("alloc::alloc() returned null.".to_string());
    }

    Ok(ptr)
}

#[test]
fn Alloc() {
    unsafe {
        let ptr: *mut usize = new(3).unwrap();
        *ptr = 1;
        *ptr.add(1) = 2;
        *ptr.add(2) = 3;

        assert_eq!(*ptr, 1);
        assert_eq!(*ptr.add(1), 2);
        assert_eq!(*ptr.add(2), 3);

        free(ptr, 3);
    }
}

#[inline(always)]
pub unsafe fn free<X>(x: *mut X, Z: usize) {
    let lay = Layout::from_size_align(size_of::<X>() * Z, align_of::<X>())
        .map_err(|e| format!("{e}"))
        .expect("invalid layout in free");
    unsafe {
        alloc::dealloc(x as *mut u8, lay);
    }
}

#[test]
fn current_alignment() {
    let test = |a: usize, b: usize| assert_eq!(1 << a.trailing_zeros(), b);
    test(128 + 1, 1);
    test(128 + 2, 2);
    test(128 + 4, 4);
    test(128 + 8, 8);
    test(128 + 16, 16);
}

#[inline(always)]
unsafe fn W<X>(x: *mut u8, y: *mut u8) {
    unsafe {
        ptr::write_unaligned(x.cast::<X>(), ptr::read_unaligned(y.cast::<X>()));
    }
}

pub unsafe fn cpy<X>(x: *mut X, y: *mut X, Z: usize) {
    /* everything into bytes */
    let mut x = x as *mut u8;
    let mut y = y as *mut u8;
    let mut Z = Z * size_of::<X>();

    macro_rules! W {
        ($x:expr, $y:expr, $T:ty) => {{
            W::<$T>(x, y);
            $x = $x.add(size_of::<$T>());
            $y = $y.add(size_of::<$T>());
            Z -= size_of::<$T>();
        }};
    }

    unsafe {
        while Z >= 128 {
            W!(x, y, ymm_t);
            W!(x, y, ymm_t);
            W!(x, y, ymm_t);
            W!(x, y, ymm_t);
        }

        while Z >= 64 {
            W!(x, y, ymm_t);
            W!(x, y, ymm_t);
        }

        while Z >= 32 {
            W!(x, y, ymm_t);
        }

        while Z >= 16 {
            W!(x, y, xmm_t);
        }

        while Z >= 8 {
            W!(x, y, u64);
        }

        while Z >= 4 {
            W!(x, y, u32);
        }

        while Z >= 2 {
            W!(x, y, u16);
        }

        while Z >= 1 {
            W!(x, y, u8);
        }
    }
}

#[test]
fn Cpy_32() {
    unsafe {
        let x = new(8).unwrap();
        let y = new(8).unwrap();

        *x = 0u64;
        *x.add(1) = 1;
        *x.add(2) = 2;
        *x.add(3) = 3;
        *x.add(4) = 4;
        *x.add(5) = 5;
        *x.add(6) = 6;
        *x.add(7) = 7;

        cpy(y, x, 8);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));
        assert_eq!(*x.add(2), *y.add(2));
        assert_eq!(*x.add(3), *y.add(3));
        assert_eq!(*x.add(4), *y.add(4));
        assert_eq!(*x.add(5), *y.add(5));
        assert_eq!(*x.add(6), *y.add(6));
        assert_eq!(*x.add(7), *y.add(7));

        free(x, 8);
        free(y, 8);
    }
}

#[test]
fn Cpy_16() {
    unsafe {
        let x = new(4).unwrap();
        let y = new(4).unwrap();

        *x = 0u64;
        *x.add(1) = 1;
        *x.add(2) = 2;
        *x.add(3) = 3;

        cpy(y, x, 4);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));
        assert_eq!(*x.add(2), *y.add(2));
        assert_eq!(*x.add(3), *y.add(3));

        free(x, 4);
        free(y, 4);
    }
}

#[test]
fn Cpy_8() {
    unsafe {
        let x: *mut u32 = new(2).unwrap();
        let y: *mut u32 = new(2).unwrap();

        *x = 0u32;
        *x.add(1) = 1;

        cpy(y, x, 2);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));

        free(x, 2);
        free(y, 2);
    }
}

#[test]
fn Cpy_4() {
    unsafe {
        let x = new(2).unwrap();
        let y = new(2).unwrap();

        *x = 0u16;
        *x.add(1) = 1;

        cpy(y, x, 2);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));

        free(x, 2);
        free(y, 2);
    }
}

#[test]
fn Cpy_2() {
    unsafe {
        let x = new(2).unwrap();
        let y = new(2).unwrap();

        *x = 0u8;
        *x.add(1) = 1;

        cpy(y, x, 2);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));

        free(x, 2);
        free(y, 2);
    }
}

#[test]
fn Cpy_1() {
    unsafe {
        let x = new(1).unwrap();
        let y = new(1).unwrap();

        *x = 0u8;

        cpy(y, x, 1);

        assert_eq!(*x, *y);

        free(x, 1);
        free(y, 1);
    }
}

#[test]
fn Cpy_16_8() {
    unsafe {
        let x = new(3).unwrap();
        let y = new(3).unwrap();

        *x = 0u64;
        *x.add(1) = 1;
        *x.add(2) = 2;

        cpy(y, x, 3);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));
        assert_eq!(*x.add(2), *y.add(2));

        free(x, 3);
        free(y, 3);
    }
}

#[test]
fn Cpy_16_4() {
    unsafe {
        let x = new(5).unwrap();
        let y = new(5).unwrap();

        *x = 0u32;
        *x.add(1) = 1;
        *x.add(2) = 2;
        *x.add(3) = 3;
        *x.add(4) = 4;

        cpy(y, x, 5);

        assert_eq!(*x, *y);
        assert_eq!(*x.add(1), *y.add(1));
        assert_eq!(*x.add(2), *y.add(2));
        assert_eq!(*x.add(3), *y.add(3));
        assert_eq!(*x.add(4), *y.add(4));

        free(x, 5);
        free(y, 5);
    }
}
