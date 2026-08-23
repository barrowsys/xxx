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

    loop {
        match Z {
            /* simd (xmm/16) */
            q if q >= 16 => {
                unsafe {
                    W::<xmm_t>(x, y);
                    x = x.add(16);
                    y = y.add(16);
                }

                Z -= 16;
            }

            /* 64 */
            q if q >= 8 => {
                unsafe {
                    W::<u64>(x, y);
                    x = x.add(8);
                    y = y.add(8);
                }

                Z -= 8;
            }

            /* 32 */
            q if q >= 4 => {
                unsafe {
                    W::<u32>(x, y);
                    x = x.add(4);
                    y = y.add(4);
                }

                Z -= 4;
            }

            /* 16 */
            q if q >= 2 => {
                unsafe {
                    W::<u16>(x, y);
                    x = x.add(2);
                    y = y.add(2);
                }

                Z -= 2;
            }

            /* 8 */
            q if q >= 1 => {
                unsafe {
                    W::<u8>(x, y);
                    x = x.add(1);
                    y = y.add(1);
                }

                Z -= 1;
            }

            _ => break,
        }
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
