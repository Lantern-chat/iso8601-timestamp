#![allow(unused)]

// borrows technique from https://github.com/rust-lang/hashbrown/pull/209
#[inline]
#[cold]
fn cold() {}

#[rustfmt::skip]
#[inline(always)]
pub unsafe fn likely(b: bool) -> bool {
    if !b { cold() } b
}

#[rustfmt::skip]
#[inline(always)]
pub unsafe fn unlikely(b: bool) -> bool {
    if b { cold() } b
}

#[rustfmt::skip]
#[inline(always)]
pub unsafe fn assume(b: bool) {
    if !b { core::hint::unreachable_unchecked() }
}

#[rustfmt::skip]
macro_rules! likely {
    ($e:expr) => {{
        #[allow(unused_unsafe)]
        // SAFETY: likely is a no-op except to codegen
        unsafe { $crate::macros::likely($e) }
    }};
}

#[rustfmt::skip]
macro_rules! unlikely {
    ($e:expr) => {{
        #[allow(unused_unsafe)]
        // SAFETY: unlikely is a no-op except to codegen
        unsafe { $crate::macros::unlikely($e) }
    }};
}

#[rustfmt::skip]
macro_rules! assume {
    ($e:expr) => { $crate::macros::assume($e) }
}

#[allow(unused_macros)]
macro_rules! import_intrinsics {
    (x86::{$($intr:ident),*}) => {
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::{$($intr),*};
        #[cfg(target_arch = "x86")]
        use core::arch::x86::{$($intr),*};
    };
}
