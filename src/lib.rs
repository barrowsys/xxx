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

#[inline(always)]
unsafe fn W<X>(x: *mut u8, y: *mut u8) {
    unsafe {
        ptr::write(x.cast::<X>(), ptr::read_unaligned(y.cast::<X>()));
    }
}

struct MemIter {
    start: *mut u8,
    length: usize,
}
impl MemIter {
    fn is_aligned_to(&self, align: usize) -> bool {
        self.start.align_offset(align) == 0
    }
    fn next_chunk_size(&self) -> usize {
        let prev_power_of_two = (self.length + 1).next_power_of_two() >> 1;
        prev_power_of_two.min(32)
    }
}
impl Iterator for MemIter {
    type Item = MemIter; // dwai

    fn next(&mut self) -> Option<Self::Item> {
        if self.length == 0 {
            return None;
        }
        // see #[test] current_alignment()
        let current_alignment = 1 << self.start.addr().trailing_zeros();
        let max_alignment = self.next_chunk_size();
        let item = MemIter {
            start: self.start,
            length: std::cmp::min(current_alignment, max_alignment),
        };
        unsafe { // voice of god says this upholds all relevant invariants
            self.start = self.start.add(item.length);
            self.length -= item.length;
        }
        Some(item)
    }
}

#[test]
fn current_alignment() {
    let test = |a:usize, b:usize| assert_eq!(1 << a.trailing_zeros(), b);
    test(128 + 1, 1);
    test(128 + 2, 2);
    test(128 + 4, 4);
    test(128 + 8, 8);
    test(128 + 16, 16);
}

#[test]
fn mem_iter() {
    let test = |a: (usize, usize), b: &[usize]| {
        assert!(a.0 < 256);
        assert!(a.1 < 256);
        let stupid_chud_pointer: *mut u8 = &mut 0;
        let stupid_chud_pointer = stupid_chud_pointer.with_addr(0xFF00 + a.0);
        let mut iter = MemIter {
            start: stupid_chud_pointer,
            length: a.1,
        };
        let collected = iter.map(|m| m.length).collect::<Vec<_>>();
        assert_eq!(collected, b);
    };
    test((0, 16), &[
        16, // 0x00 -> copy 16 -> total 16
    ]);
    test((1, 4), &[
        1, // 0x01 -> copy 1 -> total 1
        2, // 0x02 -> copy 2 -> total 3
        1, // 0x04 -> copy 1 -> total 4
    ]);
    test((1, 10), &[
        1, // 0x01 -> copy 1 -> total 1
        2, // 0x02 -> copy 2 -> total 3
        4, // 0x04 -> copy 4 -> total 7
        2, // 0x08 -> copy 2 -> total 9
        1, // 0x0A -> copy 1 -> total 10
    ]);
    test((0, 3), &[
        2, // 0x00 -> copy 2 -> total 2
        1, // 0x02 -> copy 1 -> total 3
    ]);
    test((1,34), &[
        1, // 0x01 -> copy 1 -> total 1
        2, // 0x02 -> copy 2 -> total 3
        4, // 0x04 -> copy 4 -> total 7
        8, // 0x08 -> copy 8 -> total 15
        16, // 0x10 -> copy 16 -> total 31
        2, // 0x20 -> copy 2 -> total 33
        1, // 0x22 -> copy 1 -> total 34
    ]);
}

pub unsafe fn cpy<X>(x: *mut X, y: *mut X, Z: usize) {
    /* everything into bytes */
    let mut x = x as *mut u8;
    let mut iter = MemIter {
        start: y as *mut u8,
        length: Z * size_of::<X>(),
    };

    while let Some(Z) = iter.next() {
        match Z.length {
            /* simd (ymm/512) */
            q if q >= 32 => {
                unsafe {
                    W::<ymm_t>(x, Z.start);
                    x = x.add(32);
                }
            }

            /* simd (xmm/16) */
            q if q >= 16 => {
                unsafe {
                    W::<xmm_t>(x, Z.start);
                    x = x.add(16);
                }
            }

            /* 64 */
            q if q >= 8 => {
                unsafe {
                    W::<u64>(x, Z.start);
                    x = x.add(8);
                }
            }

            /* 32 */
            q if q >= 4 => {
                unsafe {
                    W::<u32>(x, Z.start);
                    x = x.add(4);
                }
            }

            /* 16 */
            q if q >= 2 => {
                unsafe {
                    W::<u16>(x, Z.start);
                    x = x.add(2);
                }
            }

            /* 8 */
            q if q >= 1 => {
                unsafe {
                    W::<u8>(x, Z.start);
                    x = x.add(1);
                }
            }

            _ => {},
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
