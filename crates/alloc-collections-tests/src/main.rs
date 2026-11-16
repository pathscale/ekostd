//@ mode: run
//@ exit: 0
//! Unit tests for the collections of the ekostd bootstrap `alloc` (`bootstrap/alloc/src/collections`): the B-tree
//! `BTreeMap` and the `BTreeSet` over it, against a model that is one sorted `Vec` of pairs (binary search, insert,
//! remove: correct by inspection), through long pseudo-random operation sequences that grow trees several levels
//! deep and shrink them back to empty.
//!
//! A test harness links std, so this is a plain `no_main` program built on the empty sysroot: it prints one line per
//! failed check and a count, and exits 0 only when every check passed. Its allocator is a bump allocator over one
//! anonymous `mmap` (the stage0 libSystem stub has no `malloc`); memory is returned only by the process exit.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::ops::Bound;
use core::sync::atomic::{AtomicUsize, Ordering};

unsafe extern "C" {
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn abort() -> !;
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
}

fn out(text: &str) {
    let mut done = 0;
    while done < text.len() {
        let n = unsafe { write(1, text.as_ptr().add(done), text.len() - done) };
        if n <= 0 {
            return;
        }
        done += n as usize;
    }
}

fn out_u64(mut value: u64) {
    let mut buf = [0u8; 20];
    let mut idx = buf.len();
    loop {
        idx -= 1;
        buf[idx] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    if let Ok(text) = core::str::from_utf8(&buf[idx..]) {
        out(text);
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    out("alloc-collections-tests: panicked");
    if let Some(location) = info.location() {
        out(" at ");
        out(location.file());
        out(":");
        out_u64(location.line() as u64);
    }
    out("\n");
    unsafe { abort() }
}

/// Bump allocation inside one anonymous mapping, reserved on first use and committed by the kernel page by page.
struct Bump;
const ARENA: usize = 2 << 30;
static BASE: AtomicUsize = AtomicUsize::new(0);
static NEXT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Bump {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut base = BASE.load(Ordering::Relaxed);
        if base == 0 {
            // PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON (Darwin).
            let p = unsafe { mmap(core::ptr::null_mut(), ARENA, 3, 0x0002 | 0x1000, -1, 0) };
            if p as usize == usize::MAX {
                return core::ptr::null_mut();
            }
            base = p as usize;
            BASE.store(base, Ordering::Relaxed);
            NEXT.store(base, Ordering::Relaxed);
        }
        let start = (NEXT.load(Ordering::Relaxed) + layout.align() - 1) & !(layout.align() - 1);
        if start + layout.size() > base + ARENA {
            return core::ptr::null_mut();
        }
        NEXT.store(start + layout.size(), Ordering::Relaxed);
        start as *mut u8
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // The most recent allocation is given back; anything else stays until exit.
        if ptr as usize + layout.size() == NEXT.load(Ordering::Relaxed) {
            NEXT.store(ptr as usize, Ordering::Relaxed);
        }
    }
}

#[global_allocator]
static HEAP: Bump = Bump;

struct Run {
    passed: u64,
    failed: u64,
}

impl Run {
    fn check(&mut self, ok: bool, what: &str, step: u64) {
        if ok {
            self.passed += 1;
        } else {
            self.failed += 1;
            if self.failed <= 40 {
                out("FAIL ");
                out(what);
                out(" at step ");
                out_u64(step);
                out("\n");
            }
        }
    }
}

/// xorshift64: the operation sequence is reproducible from the seed.
struct Rng(u64);
impl Rng {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

/// The reference: one sorted Vec of pairs.
struct Model {
    pairs: Vec<(u32, u64)>,
}

impl Model {
    fn find(&self, key: u32) -> Result<usize, usize> { self.pairs.binary_search_by(|(k, _)| k.cmp(&key)) }
    fn insert(&mut self, key: u32, value: u64) -> Option<u64> {
        match self.find(key) {
            Ok(i) => Some(core::mem::replace(&mut self.pairs[i].1, value)),
            Err(i) => {
                self.pairs.insert(i, (key, value));
                None
            }
        }
    }
    fn remove(&mut self, key: u32) -> Option<u64> {
        match self.find(key) {
            Ok(i) => Some(self.pairs.remove(i).1),
            Err(_) => None,
        }
    }
    fn get(&self, key: u32) -> Option<u64> { self.find(key).ok().map(|i| self.pairs[i].1) }
    /// Pairs with lo <= key < hi.
    fn span(&self, lo: u32, hi: u32) -> &[(u32, u64)] {
        let a = self.pairs.partition_point(|(k, _)| *k < lo);
        let b = self.pairs.partition_point(|(k, _)| *k < hi);
        &self.pairs[a..if b < a { a } else { b }]
    }
}

fn same(run: &mut Run, map: &BTreeMap<u32, u64>, model: &Model, step: u64) {
    run.check(map.len() == model.pairs.len(), "len", step);
    run.check(map.iter().map(|(k, v)| (*k, *v)).eq(model.pairs.iter().copied()), "iter order", step);
    run.check(map.iter().rev().map(|(k, v)| (*k, *v)).eq(model.pairs.iter().rev().copied()), "iter rev", step);
    run.check(map.keys().copied().eq(model.pairs.iter().map(|(k, _)| *k)), "keys", step);
    run.check(map.values().copied().eq(model.pairs.iter().map(|(_, v)| *v)), "values", step);
    run.check(map.first_key_value().map(|(k, v)| (*k, *v)) == model.pairs.first().copied(), "first", step);
    run.check(map.last_key_value().map(|(k, v)| (*k, *v)) == model.pairs.last().copied(), "last", step);
}

fn session(run: &mut Run, seed: u64, span: u64, steps: u64) {
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut map = BTreeMap::new();
    let mut model = Model { pairs: Vec::new() };
    let mut step = 0;
    while step < steps {
        let key = rng.below(span) as u32;
        let value = rng.below(1 << 40);
        match rng.below(16) {
            0..=6 => run.check(map.insert(key, value) == model.insert(key, value), "insert", step),
            7..=9 => run.check(map.remove(&key) == model.remove(key), "remove", step),
            10 => {
                let want = if model.pairs.is_empty() { None } else { Some(model.pairs.remove(0)) };
                run.check(map.pop_first() == want, "pop_first", step);
            }
            11 => run.check(map.pop_last() == model.pairs.pop(), "pop_last", step),
            12 => {
                *map.entry(key).or_insert(0) += value % 5;
                let now = model.get(key).unwrap_or(0) + value % 5;
                model.insert(key, now);
            }
            13 => {
                let lo = rng.below(span) as u32;
                let hi = lo + rng.below(span / 4 + 2) as u32;
                let want = model.span(lo, hi);
                run.check(map.range(lo..hi).map(|(k, v)| (*k, *v)).eq(want.iter().copied()), "range", step);
                run.check(map.range(lo..hi).rev().map(|(k, v)| (*k, *v)).eq(want.iter().rev().copied()), "range rev", step);
                let want = model.span(lo + 1, hi + 1);
                run.check(map.range((Bound::Excluded(lo), Bound::Included(hi))).map(|(k, v)| (*k, *v)).eq(want.iter().copied()), "range excluded..=", step);
                let want = model.span(0, hi);
                run.check(map.range(..hi).map(|(k, v)| (*k, *v)).eq(want.iter().copied()), "range ..hi", step);
                for (_, v) in map.range_mut(lo..hi) {
                    *v += 1;
                }
                let a = model.pairs.partition_point(|(k, _)| *k < lo);
                let b = model.pairs.partition_point(|(k, _)| *k < hi);
                let mut i = a;
                while i < b {
                    model.pairs[i].1 += 1;
                    i += 1;
                }
            }
            14 => {
                run.check(map.get(&key).copied() == model.get(key), "get", step);
                run.check(map.contains_key(&key) == model.get(key).is_some(), "contains_key", step);
                if let Some(v) = map.get_mut(&key) {
                    *v ^= 3;
                }
                if let Ok(i) = model.find(key) {
                    model.pairs[i].1 ^= 3;
                }
            }
            _ => {
                if step % 97 == 0 {
                    map.retain(|k, v| {
                        *v += 1;
                        k % 3 != 0
                    });
                    model.pairs.retain_mut(|(k, v)| {
                        *v += 1;
                        *k % 3 != 0
                    });
                } else if step % 8 == 0 {
                    // Both ends at once, in a random interleaving, with the remaining length.
                    let mut it = map.iter();
                    let (mut front, mut back) = (0usize, model.pairs.len());
                    loop {
                        let got = if rng.below(2) == 0 {
                            let want = if front < back { front += 1; Some(model.pairs[front - 1]) } else { None };
                            it.next().map(|(k, v)| (*k, *v)) == want
                        } else {
                            let want = if front < back { back -= 1; Some(model.pairs[back]) } else { None };
                            it.next_back().map(|(k, v)| (*k, *v)) == want
                        };
                        run.check(got && it.len() == back - front, "double-ended iter", step);
                        if front == back {
                            run.check(it.next().is_none() && it.next_back().is_none(), "iter end", step);
                            break;
                        }
                    }
                }
            }
        }
        if step % 1000 == 0 {
            same(run, &map, &model, step);
        }
        if step % 5000 == 0 {
            let mut other = BTreeMap::new();
            let mut count = rng.below(300);
            while count > 0 {
                let k = rng.below(span) as u32;
                other.insert(k, 1);
                model.insert(k, 1);
                count -= 1;
            }
            map.append(&mut other);
            run.check(other.is_empty(), "append empties", step);
            same(run, &map, &model, step);
            let copy = map.clone();
            run.check(copy == map, "clone eq", step);
            let mut odd = false;
            for (_, v) in map.iter_mut().rev() {
                if odd {
                    *v += 9;
                }
                odd = !odd;
            }
            let mut odd = false;
            let mut i = model.pairs.len();
            while i > 0 {
                i -= 1;
                if odd {
                    model.pairs[i].1 += 9;
                }
                odd = !odd;
            }
            same(run, &map, &model, step);
        }
        step += 1;
    }
    same(run, &map, &model, step);
    run.check(map.clone().into_iter().eq(model.pairs.iter().copied()), "into_iter", step);
    run.check(map.clone().into_keys().eq(model.pairs.iter().map(|(k, _)| *k)), "into_keys", step);
    while let Some(pair) = map.pop_last() {
        run.check(Some(pair) == model.pairs.pop(), "drain by pop_last", step);
    }
    run.check(map.is_empty() && map.iter().next().is_none() && model.pairs.is_empty(), "empty at end", step);
}

fn sets(run: &mut Run) {
    let mut set = BTreeSet::new();
    let mut i = 1000u32;
    while i > 0 {
        i -= 1;
        set.insert(i * 7 % 1013);
    }
    run.check(set.len() == 1000, "set len", 0);
    run.check(set.iter().zip(set.iter().skip(1)).all(|(a, b)| a < b), "set ascending", 0);
    run.check((&set).into_iter().count() == 1000, "set &into_iter", 0);
    run.check(set.range(10..20).count() == (10..20).filter(|x| set.contains(x)).count(), "set range", 0);
    let mut other = BTreeSet::new();
    other.insert(5000);
    other.insert(3);
    set.append(&mut other);
    run.check(set.len() == 1001 && set.contains(&5000) && other.is_empty(), "set append", 0);
    run.check(set.first() == Some(&0) && set.last() == Some(&5000), "set first last", 0);
}

fn sorts(run: &mut Run) {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(200_000);
    let mut i = 0u32;
    while i < 200_000 {
        pairs.push((rng.below(1000) as u32, i));
        i += 1;
    }
    pairs.sort_by_key(|p| p.0);
    run.check(
        pairs.len() == 200_000 && pairs.iter().zip(pairs.iter().skip(1)).all(|(a, b)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)),
        "sort_by_key stable",
        0,
    );

    let mut values: Vec<u64> = Vec::with_capacity(200_000);
    let mut sum = 0u64;
    let mut xor = 0u64;
    let mut n = 0u32;
    while n < 200_000 {
        let v = rng.below(1 << 40);
        sum = sum.wrapping_add(v);
        xor ^= v;
        values.push(v);
        n += 1;
    }
    values.sort_unstable();
    let mut sum_after = 0u64;
    let mut xor_after = 0u64;
    for v in values.iter() {
        sum_after = sum_after.wrapping_add(*v);
        xor_after ^= *v;
    }
    run.check(
        values.len() == 200_000
            && values.iter().zip(values.iter().skip(1)).all(|(a, b)| a <= b)
            && sum == sum_after
            && xor == xor_after,
        "sort_unstable",
        0,
    );

    let mut reversed: Vec<u32> = Vec::with_capacity(100_000);
    let mut k = 100_000u32;
    while k > 0 {
        k -= 1;
        reversed.push(k);
    }
    reversed.sort();
    run.check(reversed.iter().copied().eq(0..100_000), "sort reversed", 0);

    let mut sorted: Vec<u32> = Vec::with_capacity(100_000);
    let mut k = 0u32;
    while k < 100_000 {
        sorted.push(k);
        k += 1;
    }
    sorted.sort();
    run.check(sorted.iter().copied().eq(0..100_000), "sort sorted", 0);

    let mut equal: Vec<(u32, u32)> = Vec::with_capacity(50_000);
    let mut k = 0u32;
    while k < 50_000 {
        equal.push((7, k));
        k += 1;
    }
    equal.sort_by_key(|p| p.0);
    run.check(equal.iter().all(|p| p.0 == 7) && equal.iter().map(|p| p.1).eq(0..50_000), "sort all equal", 0);

    let mut empty: Vec<u32> = Vec::new();
    empty.sort();
    empty.sort_unstable();
    let mut one: Vec<u32> = Vec::new();
    one.push(5);
    one.sort();
    one.sort_unstable();
    run.check(empty.is_empty() && one.len() == 1 && one[0] == 5, "sort tiny", 0);
}

fn rcs(run: &mut Run) {
    struct D<'a>(&'a core::cell::Cell<u32>);
    impl Clone for D<'_> {
        fn clone(&self) -> Self { D(self.0) }
    }
    impl Drop for D<'_> {
        fn drop(&mut self) { self.0.set(self.0.get() + 1); }
    }

    // from_raw takes back the reference into_raw gave out, sharing the counts of the same allocation.
    let shared = core::cell::Cell::new(0u32);
    let a = alloc::rc::Rc::new(D(&shared));
    let b = a.clone();
    let p = alloc::rc::Rc::into_raw(a);
    let c = unsafe { alloc::rc::Rc::from_raw(p) };
    run.check(alloc::rc::Rc::strong_count(&b) == 2, "rc from_raw shares the count", 0);
    run.check(alloc::rc::Rc::ptr_eq(&b, &c), "rc from_raw shares the count", 1);
    drop(c);
    run.check(shared.get() == 0 && alloc::rc::Rc::strong_count(&b) == 1, "rc from_raw shares the count", 2);
    drop(b);
    run.check(shared.get() == 1, "rc from_raw shares the count", 3);

    // A Weak upgrades while a strong reference lives and not after the value is dropped.
    let weak_drops = core::cell::Cell::new(0u32);
    let r = alloc::rc::Rc::new(D(&weak_drops));
    let w = alloc::rc::Rc::downgrade(&r);
    run.check(alloc::rc::Rc::weak_count(&r) == 1 && alloc::rc::Rc::strong_count(&r) == 1, "rc weak", 0);
    run.check(w.upgrade().is_some(), "rc weak", 1);
    run.check(alloc::rc::Rc::strong_count(&r) == 1 && w.strong_count() == 1 && w.weak_count() == 1, "rc weak", 2);
    drop(r);
    run.check(weak_drops.get() == 1, "rc weak", 3);
    run.check(w.upgrade().is_none() && w.strong_count() == 0 && w.weak_count() == 0, "rc weak", 4);
    drop(w);
    run.check(weak_drops.get() == 1, "rc weak", 5);

    // try_unwrap fails while another strong reference lives; with only a Weak left it moves the value out.
    let unwrap_drops = core::cell::Cell::new(0u32);
    let r = alloc::rc::Rc::new(D(&unwrap_drops));
    let r2 = r.clone();
    let r = match alloc::rc::Rc::try_unwrap(r) {
        Ok(_) => {
            run.check(false, "rc try_unwrap", 0);
            r2.clone()
        }
        Err(back) => back,
    };
    run.check(alloc::rc::Rc::strong_count(&r) == 2 && unwrap_drops.get() == 0, "rc try_unwrap", 1);
    drop(r2);
    let w = alloc::rc::Rc::downgrade(&r);
    match alloc::rc::Rc::try_unwrap(r) {
        Ok(value) => {
            run.check(unwrap_drops.get() == 0, "rc try_unwrap", 2);
            run.check(w.upgrade().is_none(), "rc try_unwrap", 3);
            drop(value);
            run.check(unwrap_drops.get() == 1, "rc try_unwrap", 4);
        }
        Err(_) => run.check(false, "rc try_unwrap", 5),
    }
    drop(w);
    run.check(unwrap_drops.get() == 1, "rc try_unwrap", 6);

    // make_mut with a Weak alive moves the value into a new allocation: no clone, no drop, the Weak is cut off.
    let make_drops = core::cell::Cell::new(0u32);
    let mut r = alloc::rc::Rc::new(D(&make_drops));
    let w = alloc::rc::Rc::downgrade(&r);
    let _ = alloc::rc::Rc::make_mut(&mut r);
    run.check(make_drops.get() == 0, "rc make_mut weak", 0);
    run.check(w.upgrade().is_none(), "rc make_mut weak", 1);
    run.check(alloc::rc::Rc::strong_count(&r) == 1 && alloc::rc::Rc::weak_count(&r) == 0, "rc make_mut weak", 2);
    drop(w);
    run.check(make_drops.get() == 0, "rc make_mut weak", 3);
    drop(r);
    run.check(make_drops.get() == 1, "rc make_mut weak", 4);

    // make_mut with a second strong reference clones; the other keeps the old value.
    let mut a = alloc::rc::Rc::new(5u32);
    let b = a.clone();
    *alloc::rc::Rc::make_mut(&mut a) = 7;
    run.check(*a == 7 && *b == 5, "rc make_mut shared", 0);
    run.check(!alloc::rc::Rc::ptr_eq(&a, &b), "rc make_mut shared", 1);
    run.check(alloc::rc::Rc::strong_count(&a) == 1 && alloc::rc::Rc::strong_count(&b) == 1, "rc make_mut shared", 2);

    // Weak::new points at no allocation.
    let w = alloc::rc::Weak::<u32>::new();
    let w2 = w.clone();
    run.check(w.upgrade().is_none(), "rc weak new", 0);
    run.check(w.strong_count() == 0 && w.weak_count() == 0, "rc weak new", 1);
    run.check(w.ptr_eq(&w2), "rc weak new", 2);
    drop(w);
    drop(w2);

    // get_mut only while no other strong reference and no Weak exists.
    let mut a = alloc::rc::Rc::new(1u32);
    run.check(alloc::rc::Rc::get_mut(&mut a).is_some(), "rc get_mut", 0);
    let w = alloc::rc::Rc::downgrade(&a);
    run.check(alloc::rc::Rc::get_mut(&mut a).is_none(), "rc get_mut", 1);
    drop(w);
    if let Some(v) = alloc::rc::Rc::get_mut(&mut a) {
        *v = 2;
    }
    run.check(*a == 2, "rc get_mut", 2);

    // into_inner and unwrap_or_clone.
    let a = alloc::rc::Rc::new(3u32);
    let b = a.clone();
    run.check(alloc::rc::Rc::into_inner(a).is_none(), "rc into_inner", 0);
    run.check(alloc::rc::Rc::into_inner(b) == Some(3), "rc into_inner", 1);
    let a = alloc::rc::Rc::new(4u32);
    let b = a.clone();
    run.check(alloc::rc::Rc::unwrap_or_clone(a) == 4 && alloc::rc::Rc::strong_count(&b) == 1, "rc unwrap_or_clone", 0);
    run.check(alloc::rc::Rc::unwrap_or_clone(b) == 4, "rc unwrap_or_clone", 1);

    // increment_strong_count / decrement_strong_count through the raw pointer.
    let count_drops = core::cell::Cell::new(0u32);
    let p = alloc::rc::Rc::into_raw(alloc::rc::Rc::new(D(&count_drops)));
    unsafe { alloc::rc::Rc::increment_strong_count(p) };
    let a = unsafe { alloc::rc::Rc::from_raw(p) };
    run.check(alloc::rc::Rc::strong_count(&a) == 2, "rc strong count raw", 0);
    unsafe { alloc::rc::Rc::decrement_strong_count(p) };
    run.check(alloc::rc::Rc::strong_count(&a) == 1 && count_drops.get() == 0, "rc strong count raw", 1);
    drop(a);
    run.check(count_drops.get() == 1, "rc strong count raw", 2);

    // Many clone/drop pairs leave the count where it started.
    let r = alloc::rc::Rc::new(1u32);
    let mut i = 0u32;
    while i < 100_000 {
        let x = r.clone();
        drop(x);
        i += 1;
    }
    run.check(alloc::rc::Rc::strong_count(&r) == 1 && alloc::rc::Rc::weak_count(&r) == 0, "rc clone drop pairs", 0);
}

fn heaps(run: &mut Run) {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let mut values: Vec<u64> = Vec::with_capacity(100_000);
    let mut sum = 0u64;
    let mut xor = 0u64;
    let mut n = 0u32;
    while n < 100_000 {
        let v = rng.below(1 << 20);
        sum = sum.wrapping_add(v);
        xor ^= v;
        values.push(v);
        n += 1;
    }
    let mut heap = alloc::collections::BinaryHeap::from(values.clone());
    run.check(heap.len() == 100_000, "heap from vec len", 0);
    let mut prev = u64::MAX;
    let mut ordered = true;
    let mut count = 0u32;
    let mut sum_after = 0u64;
    let mut xor_after = 0u64;
    while let Some(v) = heap.pop() {
        if v > prev {
            ordered = false;
        }
        prev = v;
        count += 1;
        sum_after = sum_after.wrapping_add(v);
        xor_after ^= v;
    }
    run.check(
        ordered && count == 100_000 && sum_after == sum && xor_after == xor && heap.is_empty(),
        "heap from vec pops sorted",
        0,
    );

    let collected: alloc::collections::BinaryHeap<u64> = values.iter().copied().collect();
    let sorted = collected.into_sorted_vec();
    values.sort_unstable();
    run.check(
        sorted.len() == 100_000 && sorted.iter().zip(sorted.iter().skip(1)).all(|(a, b)| a <= b) && sorted.iter().eq(values.iter()),
        "heap into_sorted_vec",
        0,
    );

    let mut small = alloc::collections::BinaryHeap::from([5u32, 9, 3]);
    {
        let mut top = small.peek_mut().unwrap();
        run.check(*top == 9, "heap peek_mut top", 0);
        *top = 1;
    }
    let p1 = small.pop();
    let p2 = small.pop();
    let p3 = small.pop();
    let p4 = small.pop();
    run.check(p1 == Some(5) && p2 == Some(3) && p3 == Some(1) && p4.is_none(), "heap peek_mut", 0);
    let mut other = alloc::collections::BinaryHeap::from([4u32, 8, 6, 2]);
    let top = other.peek_mut().map(alloc::collections::binary_heap::PeekMut::pop);
    run.check(top == Some(8) && other.len() == 3 && other.peek() == Some(&6), "heap PeekMut::pop", 0);
    let mut none: alloc::collections::BinaryHeap<u32> = alloc::collections::BinaryHeap::new();
    run.check(none.peek_mut().is_none(), "heap peek_mut empty", 0);

    let mut a: alloc::collections::BinaryHeap<u32> = alloc::collections::BinaryHeap::new();
    a.extend([7u32, 1, 12, 4]);
    a.extend(&[3u32, 10]);
    let mut b: alloc::collections::BinaryHeap<u32> = (20u32..30).collect();
    a.append(&mut b);
    run.check(a.len() == 16 && b.is_empty() && a.peek() == Some(&29), "heap misc extend append", 0);
    run.check(a.iter().copied().sum::<u32>() == 282 && (&a).into_iter().count() == 16, "heap misc iter", 0);
    let mut evens = a.clone();
    evens.retain(|x| x % 2 == 0);
    let mut kept: Vec<u32> = Vec::new();
    while let Some(v) = evens.pop() {
        kept.push(v);
    }
    run.check(kept.iter().eq([28u32, 26, 24, 22, 20, 12, 10, 4].iter()), "heap misc retain", 0);
    let mut d = a.clone();
    let drained = d.drain().count();
    run.check(drained == 16 && d.is_empty() && a.len() == 16, "heap misc drain", 0);
    a.clear();
    run.check(a.is_empty() && a.len() == 0 && a.peek().is_none(), "heap misc clear", 0);
    let roomy: alloc::collections::BinaryHeap<u32> = alloc::collections::BinaryHeap::with_capacity(10);
    run.check(roomy.capacity() >= 10 && roomy.is_empty(), "heap misc with_capacity", 0);
}

fn arc_layouts(run: &mut Run) {
    // ArcInner starts with AtomicUsize counts; no value below needs a larger alignment.
    let align = core::mem::align_of::<AtomicUsize>();
    let top = || (NEXT.load(Ordering::Relaxed) + align - 1) & !(align - 1);

    let start = top();
    drop(alloc::sync::Arc::new(7u8));
    let u8_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(u8_back, "arc u8 dealloc layout", 0);

    let start = top();
    drop(alloc::sync::Arc::new([1u8; 3]));
    let array_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(array_back, "arc u8 dealloc layout", 1);

    let start = top();
    drop(alloc::sync::Arc::new(5u16));
    let u16_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(u16_back, "arc u8 dealloc layout", 2);

    let start = top();
    drop(alloc::sync::Arc::<[u8]>::from(&[1u8, 2, 3][..]));
    let slice_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(slice_back, "arc slice dealloc layout", 0);

    let start = top();
    drop(alloc::sync::Arc::<str>::from("abcde"));
    let str_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(str_back, "arc slice dealloc layout", 1);

    let start = top();
    let strong = alloc::sync::Arc::new(9u8);
    let weak = alloc::sync::Arc::downgrade(&strong);
    drop(strong);
    let held = NEXT.load(Ordering::Relaxed) > start;
    drop(weak);
    let weak_back = NEXT.load(Ordering::Relaxed) == start;
    run.check(held && weak_back, "arc weak dealloc layout", 0);
}

fn vecdeques(run: &mut Run) {
    // push_front 0..N leaves N-1, ..., 0 front to back, so pop_back yields 0, 1, 2, ...
    const N: u32 = 1_000_000;
    let mut dq = alloc::collections::VecDeque::new();
    let mut i = 0u32;
    while i < N {
        dq.push_front(i);
        i += 1;
    }
    let mut ok = dq.len() == N as usize && dq.front() == Some(&(N - 1)) && dq.back() == Some(&0);
    let mut want = 0u32;
    while let Some(v) = dq.pop_back() {
        ok &= v == want;
        want += 1;
    }
    run.check(ok && want == N && dq.is_empty(), "vecdeque front-to-back queue", 0);

    let mut dq = alloc::collections::VecDeque::new();
    let mut i = 0u32;
    while i < N {
        dq.push_back(i);
        i += 1;
    }
    let mut ok = dq.len() == N as usize;
    let mut want = 0u32;
    while let Some(v) = dq.pop_front() {
        ok &= v == want;
        want += 1;
    }
    run.check(ok && want == N && dq.is_empty(), "vecdeque fifo", 0);

    // Random operations against a Vec model, pushes outnumbering pops so the deque grows, wraps and shrinks.
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let mut dq: alloc::collections::VecDeque<u64> = alloc::collections::VecDeque::new();
    let mut model: alloc::vec::Vec<u64> = alloc::vec::Vec::new();
    let mut step = 0u64;
    while step < 20_000 {
        let value = step;
        match rng.below(10) {
            0 | 1 => {
                dq.push_front(value);
                model.insert(0, value);
            }
            2 | 3 => {
                dq.push_back(value);
                model.push(value);
            }
            4 => {
                let want = if model.is_empty() { None } else { Some(model.remove(0)) };
                run.check(dq.pop_front() == want, "vecdeque model pop_front", step);
            }
            5 => run.check(dq.pop_back() == model.pop(), "vecdeque model pop_back", step),
            6 => {
                let at = rng.below(model.len() as u64 + 1) as usize;
                dq.insert(at, value);
                model.insert(at, value);
            }
            7 => {
                if model.is_empty() {
                    run.check(dq.remove(0).is_none(), "vecdeque model remove empty", step);
                } else {
                    let at = rng.below(model.len() as u64) as usize;
                    run.check(dq.remove(at) == Some(model.remove(at)), "vecdeque model remove", step);
                }
            }
            8 => {
                if !model.is_empty() {
                    let at = rng.below(model.len() as u64) as usize;
                    run.check(dq[at] == model[at], "vecdeque model index", step);
                }
            }
            _ => {
                let at = rng.below(model.len() as u64 + 2) as usize;
                run.check(dq.get(at) == model.get(at), "vecdeque model get", step);
            }
        }
        step += 1;
        if step % 1000 == 0 {
            let (a, b) = dq.as_slices();
            let slices = a.len() + b.len() == model.len() && a == &model[..a.len()] && b == &model[a.len()..];
            let same = dq.len() == model.len()
                && dq.iter().eq(model.iter())
                && dq.iter().rev().eq(model.iter().rev())
                && slices;
            run.check(same, "vecdeque model", step);
        }
    }

    // Capacity 4: 1 2 3, pop two, then 4 5 6 run past the end of the buffer.
    let mut dq = alloc::collections::VecDeque::with_capacity(4);
    dq.push_back(1u32);
    dq.push_back(2);
    dq.push_back(3);
    dq.pop_front();
    dq.pop_front();
    dq.push_back(4);
    dq.push_back(5);
    dq.push_back(6);
    let order = dq.len() == 4 && dq.iter().eq([3u32, 4, 5, 6].iter()) && dq[0] == 3 && dq[3] == 6;
    let contiguous = &*dq.make_contiguous() == &[3u32, 4, 5, 6][..];
    let one_slice = dq.as_slices().1.is_empty() && dq.as_slices().0 == &[3u32, 4, 5, 6][..];
    let as_vec = alloc::vec::Vec::from(dq) == [3u32, 4, 5, 6];
    run.check(order && contiguous && one_slice && as_vec, "vecdeque wrap", 0);

    let mut dq: alloc::collections::VecDeque<u32> = (0..10).collect();
    dq.retain(|x| *x % 3 != 0);
    let retained = dq.iter().eq([1u32, 2, 4, 5, 7, 8].iter());
    dq.swap(0, 5);
    dq.truncate(4);
    let rest = dq.iter().eq([8u32, 2, 4, 5].iter()) && dq.contains(&4) && !dq.contains(&7);
    run.check(retained && rest, "vecdeque retain swap truncate contains", 0);

    struct Counted<'a>(&'a core::cell::Cell<u32>);
    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = core::cell::Cell::new(0u32);
    {
        let mut dq = alloc::collections::VecDeque::new();
        let mut i = 0;
        while i < 100 {
            if i % 2 == 0 {
                dq.push_back(Counted(&drops));
            } else {
                dq.push_front(Counted(&drops));
            }
            i += 1;
        }
        let mut j = 0;
        while j < 30 {
            if j % 2 == 0 {
                drop(dq.pop_front());
            } else {
                drop(dq.pop_back());
            }
            j += 1;
        }
        run.check(drops.get() == 30 && dq.len() == 70, "vecdeque drops popped", 0);
    }
    run.check(drops.get() == 100, "vecdeque drops", 0);
}

fn strings_api(run: &mut Run) {
    fn shown_debug<T: core::fmt::Debug>(value: &T) -> alloc::string::String {
        let mut text = alloc::string::String::new();
        let _ = core::fmt::Write::write_fmt(&mut text, format_args!("{:?}", value));
        text
    }
    fn shown<T: core::fmt::Display>(value: &T) -> alloc::string::String {
        let mut text = alloc::string::String::new();
        let _ = core::fmt::Write::write_fmt(&mut text, format_args!("{}", value));
        text
    }

    let mut s = alloc::string::String::from("a,b;c");
    s.retain(|c| c.is_alphabetic());
    run.check(s == "abc", "string retain", 0);
    let mut s = alloc::string::String::from("h\u{e9}llo");
    let right = s.split_off(3);
    run.check(s == "h\u{e9}" && right == "llo", "string split_off", 0);
    let mut s = alloc::string::String::from("ab");
    let right = s.split_off(2);
    run.check(right.is_empty() && s == "ab", "string split_off at len", 0);

    let mut s = alloc::string::String::from("hello");
    unsafe { s.as_mut_str().as_bytes_mut()[0] = b'H'; }
    run.check(s == "Hello", "string as_mut_str", 0);
    let mut s = alloc::string::String::from("abc");
    let r: &mut str = &mut s;
    run.check(r.len() == 3, "string deref_mut", 0);

    let mut s = alloc::string::String::new();
    s.extend(["x", "y"]);
    s.extend([alloc::string::String::from("z")]);
    s.extend(['q'].iter());
    s.extend([alloc::borrow::Cow::Borrowed("w")]);
    run.check(s == "xyzqw", "string extend str/String/&char/Cow", 0);

    run.check(alloc::string::String::from('c') == "c", "string from char", 0);
    run.check(alloc::string::String::from(alloc::boxed::Box::<str>::from("bx")) == "bx", "string from Box<str>", 0);
    run.check(alloc::string::String::from(alloc::borrow::Cow::Borrowed("cw")) == "cw", "string from Cow", 0);
    let parsed = match "x".parse::<alloc::string::String>() {
        Ok(text) => text == "x",
        Err(_) => false,
    };
    run.check(parsed, "string from_str", 0);

    match alloc::string::String::from_utf8(alloc::vec![0xff]) {
        Ok(_) => run.check(false, "from_utf8 error", 0),
        Err(error) => {
            run.check(
                shown_debug(&error) == "FromUtf8Error { bytes: [255], error: Utf8Error { valid_up_to: 0, error_len: Some(1) } }",
                "from_utf8 error debug",
                0,
            );
            run.check(shown(&error) == shown(&error.utf8_error()), "from_utf8 error display", 0);
        }
    }

    let decoded = match alloc::string::String::from_utf16(&[0xD834, 0xDD1E, 0x61]) {
        Ok(text) => text == "\u{1D11E}a",
        Err(_) => false,
    };
    run.check(decoded, "string from_utf16", 0);
    match alloc::string::String::from_utf16(&[0xD800]) {
        Ok(_) => run.check(false, "from_utf16 lone surrogate", 0),
        Err(error) => run.check(shown(&error) == "invalid utf-16: lone surrogate found", "from_utf16 lone surrogate", 0),
    }
    run.check(alloc::string::String::from_utf16_lossy(&[0x61, 0xD800, 0x62]) == "a\u{FFFD}b", "string from_utf16_lossy", 0);

    let leaked: &mut str = alloc::string::String::from("k").leak();
    run.check(&*leaked == "k" && leaked.len() == 1, "string leak", 0);
    run.check(alloc::string::String::from("a") == alloc::borrow::Cow::Borrowed("a"), "string eq Cow", 0);

    let mut s = alloc::string::String::with_capacity(64);
    s.push_str("ab");
    s.shrink_to_fit();
    run.check(s.capacity() == 2 && s == "ab", "string shrink_to_fit", 0);
}

fn btrees_api(run: &mut Run) {
    // split_off of 0..100 (i -> i * 10) at 60, above every key and at the least key.
    let mut left: BTreeMap<i32, i32> = (0..100).map(|i| (i, i * 10)).collect();
    let right = left.split_off(&60);
    run.check(left.len() == 60 && right.len() == 40, "split_off lengths", 0);
    run.check(right.first_key_value() == Some((&60, &600)) && left.last_key_value() == Some((&59, &590)), "split_off ends", 0);
    run.check(left.keys().copied().eq(0..60) && right.keys().copied().eq(60..100), "split_off keys", 0);
    let mut whole = left.clone();
    let none = whole.split_off(&1000);
    run.check(none.is_empty() && whole.len() == 60 && whole.keys().copied().eq(0..60), "split_off above all", 0);
    let mut all = right.clone();
    let moved = all.split_off(&0);
    run.check(all.is_empty() && moved.len() == 40 && moved.keys().copied().eq(60..100), "split_off everything", 0);

    // Both halves of a split keep working as B-trees under random inserts and removals.
    let mut low: BTreeMap<u32, u64> = (0u32..100).map(|i| (i, i as u64 * 10)).collect();
    let mut high = low.split_off(&60);
    let mut low_model = Model { pairs: (0u32..60).map(|i| (i, i as u64 * 10)).collect() };
    let mut high_model = Model { pairs: (60u32..100).map(|i| (i, i as u64 * 10)).collect() };
    same(run, &low, &low_model, 0);
    same(run, &high, &high_model, 0);
    let mut rng = Rng(0x5851_f42d_4c95_7f2d);
    let mut step = 1u64;
    while step <= 1000 {
        let key = rng.below(200) as u32;
        let value = rng.below(1 << 30);
        let (map, model) = if rng.below(2) == 0 { (&mut low, &mut low_model) } else { (&mut high, &mut high_model) };
        if rng.below(3) == 0 {
            run.check(map.remove(&key) == model.remove(key), "split halves remove", step);
        } else {
            run.check(map.insert(key, value) == model.insert(key, value), "split halves insert", step);
        }
        if step % 100 == 0 {
            same(run, &low, &low_model, step);
            same(run, &high, &high_model, step);
        }
        step += 1;
    }

    // first_entry / last_entry.
    let mut m: BTreeMap<i32, i32> = BTreeMap::new();
    m.insert(1, 10);
    m.insert(2, 20);
    match m.first_entry() {
        Some(e) => {
            run.check(*e.key() == 1 && *e.get() == 10, "first_entry key", 0);
            *e.into_mut() += 1;
        }
        None => run.check(false, "first_entry key", 1),
    }
    run.check(m[&1] == 11 && m[&2] == 20, "first_entry into_mut", 0);
    let last = m.last_entry().map(|e| e.remove());
    run.check(last == Some(20) && m.len() == 1 && !m.contains_key(&2), "last_entry remove", 0);
    let mut empty: BTreeMap<i32, i32> = BTreeMap::new();
    run.check(empty.first_entry().is_none() && empty.last_entry().is_none(), "first_entry empty", 0);

    // Extend from references, or_insert_with_key, and OccupiedEntry::insert on the last entry.
    m.extend([(&3, &4)]);
    run.check(m[&3] == 4 && m.len() == 2, "extend refs", 0);
    let got = *m.entry(5).or_insert_with_key(|k| k * 2);
    run.check(got == 10 && m[&5] == 10, "or_insert_with_key", 0);
    let kept = *m.entry(5).or_insert_with_key(|k| k * 3);
    run.check(kept == 10, "or_insert_with_key occupied", 0);
    let swapped = m.last_entry().map(|mut e| {
        let old = e.insert(50);
        (*e.key(), old, *e.get())
    });
    run.check(swapped == Some((5, 10, 50)) && m[&5] == 50, "last_entry insert", 0);

    // BTreeSet additions.
    let mut a: BTreeSet<i32> = BTreeSet::from([1, 2, 3]);
    let b: BTreeSet<i32> = BTreeSet::from([3, 4]);
    run.check(a.is_superset(&BTreeSet::from([1, 2])) && !b.is_superset(&a), "set is_superset", 0);
    run.check(a.symmetric_difference(&b).copied().eq([1, 2, 4]), "set symmetric_difference", 0);
    run.check((&a | &b).iter().copied().eq([1, 2, 3, 4]), "set bitor", 0);
    run.check((&a & &b).iter().copied().eq([3]), "set bitand", 0);
    run.check((&a - &b).iter().copied().eq([1, 2]), "set sub", 0);
    run.check((&a ^ &b).iter().copied().eq([1, 2, 4]), "set bitxor", 0);
    run.check(a.replace(2) == Some(2), "set replace", 0);
    run.check(a.replace(9).is_none() && a.len() == 4, "set replace", 1);
    let tail = a.split_off(&3);
    run.check(a.iter().copied().eq([1, 2]) && tail.iter().copied().eq([3, 9]), "set split_off", 0);
    let it: alloc::collections::btree_set::Iter<'_, i32> = a.iter();
    run.check(it.len() == 2 && it.clone().count() == 2, "set Iter", 0);
    run.check(a.iter().rev().copied().eq([2, 1]), "set Iter rev", 0);
}

fn vecs_api(run: &mut Run) {
    // extend_from_within appends clones of a range of the vector's own elements.
    let mut v: alloc::vec::Vec<i32> = alloc::vec::Vec::from([1, 2, 3]);
    v.extend_from_within(..2);
    run.check(v == [1, 2, 3, 1, 2], "vec extend_from_within", 0);
    v.extend_from_within(1..=1);
    run.check(v == [1, 2, 3, 1, 2, 2], "vec extend_from_within", 1);
    let mut full: alloc::vec::Vec<alloc::vec::Vec<i32>> = alloc::vec::Vec::with_capacity(3);
    full.push(alloc::vec::Vec::from([1]));
    full.push(alloc::vec::Vec::from([2]));
    full.push(alloc::vec::Vec::from([3]));
    run.check(full.capacity() == 3, "vec extend_from_within realloc", 0);
    full.extend_from_within(..);
    run.check(
        full.len() == 6 && full.iter().map(|x| x[0]).eq([1, 2, 3, 1, 2, 3].iter().copied()),
        "vec extend_from_within realloc",
        1,
    );

    // shrink_to keeps max(len, min_capacity) and never grows.
    let mut s: alloc::vec::Vec<i32> = alloc::vec::Vec::with_capacity(10);
    s.push(1);
    s.push(2);
    s.push(3);
    s.shrink_to(5);
    run.check(s.capacity() == 5 && s == [1, 2, 3], "vec shrink_to", 0);
    s.shrink_to(0);
    run.check(s.capacity() == 3 && s == [1, 2, 3], "vec shrink_to", 1);
    s.shrink_to(8);
    run.check(s.capacity() == 3 && s == [1, 2, 3], "vec shrink_to", 2);

    // spare_capacity_mut covers len..capacity; written slots become visible through set_len.
    let mut sp: alloc::vec::Vec<i32> = alloc::vec::Vec::with_capacity(6);
    sp.push(1);
    let cap = sp.capacity();
    let spare = sp.spare_capacity_mut();
    let spare_len = spare.len();
    spare[0].write(7);
    spare[1].write(8);
    run.check(spare_len == cap - 1, "vec spare_capacity_mut", 0);
    unsafe { sp.set_len(3) };
    run.check(sp == [1, 7, 8], "vec spare_capacity_mut", 1);

    // extract_if over the whole vector and over a sub-range.
    let mut e: alloc::vec::Vec<i32> = (1..11).collect();
    let taken: alloc::vec::Vec<i32> = e.extract_if(.., |x| *x % 2 == 0).collect();
    run.check(taken == [2, 4, 6, 8, 10] && e == [1, 3, 5, 7, 9], "vec extract_if", 0);
    let mut r: alloc::vec::Vec<i32> = (0..8).collect();
    let taken: alloc::vec::Vec<i32> = r.extract_if(2..6, |x| *x > 0).collect();
    run.check(taken == [2, 3, 4, 5] && r == [0, 1, 6, 7], "vec extract_if range", 0);

    // Dropped after one element: everything else stays, in order.
    let mut early: alloc::vec::Vec<i32> = (1..7).collect();
    {
        let mut it = early.extract_if(.., |x| *x % 2 == 0);
        let first = it.next();
        run.check(first == Some(2), "vec extract_if early drop", 0);
    }
    run.check(early == [1, 3, 4, 5, 6], "vec extract_if early drop", 1);

    // No element dropped twice or leaked.
    struct Counted<'a>(u32, &'a core::cell::Cell<u32>);
    impl Drop for Counted<'_> {
        fn drop(&mut self) { self.1.set(self.1.get() + 1); }
    }
    let drops = core::cell::Cell::new(0u32);
    let mut c: alloc::vec::Vec<Counted<'_>> = alloc::vec::Vec::new();
    let mut i = 0u32;
    while i < 8 {
        c.push(Counted(i, &drops));
        i += 1;
    }
    {
        let mut it = c.extract_if(.., |x| x.0 % 3 == 0);
        let a = it.next();
        let b = it.next();
        run.check(
            a.as_ref().map(|x| x.0) == Some(0) && b.as_ref().map(|x| x.0) == Some(3) && drops.get() == 0,
            "vec extract_if drops",
            0,
        );
        drop(a);
        drop(b);
    }
    run.check(
        drops.get() == 2 && c.len() == 6 && c.iter().map(|x| x.0).eq([1u32, 2, 4, 5, 6, 7].iter().copied()),
        "vec extract_if drops",
        1,
    );
    drop(c);
    run.check(drops.get() == 8, "vec extract_if drops", 2);

    // pop_if takes the last element only when the predicate holds.
    let mut p: alloc::vec::Vec<i32> = alloc::vec::Vec::from([1, 9]);
    run.check(p.pop_if(|x| *x > 5) == Some(9) && p == [1], "vec pop_if", 0);
    run.check(p.pop_if(|x| *x > 5).is_none() && p == [1], "vec pop_if", 1);
    let mut none: alloc::vec::Vec<i32> = alloc::vec::Vec::new();
    run.check(none.pop_if(|_| true).is_none(), "vec pop_if", 2);

    // into_flattened reuses the allocation.
    let flat = alloc::vec::Vec::from([[1, 2], [3, 4]]).into_flattened();
    run.check(flat == [1, 2, 3, 4] && flat.capacity() == 4, "vec into_flattened", 0);

    // IntoIter: clone of the remaining elements, as_mut_slice, Debug, Default.
    let mut it = alloc::vec::Vec::from([1, 2, 3]).into_iter();
    let first = it.next();
    let copy: alloc::vec::Vec<i32> = it.clone().collect();
    run.check(first == Some(1) && copy == [2, 3], "vec IntoIter clone", 0);
    it.as_mut_slice()[0] = 9;
    run.check(it.as_slice().iter().eq([9, 3].iter()), "vec IntoIter as_mut_slice", 0);
    run.check(it.next() == Some(9) && it.next() == Some(3) && it.next().is_none(), "vec IntoIter as_mut_slice", 1);
    let mut text = alloc::string::String::new();
    let shown = core::fmt::Write::write_fmt(&mut text, format_args!("{:?}", alloc::vec::Vec::from([1, 2]).into_iter()));
    run.check(shown.is_ok() && text.as_str() == "IntoIter([1, 2])", "vec IntoIter debug", 0);
    let empty: alloc::vec::IntoIter<i32> = Default::default();
    run.check(empty.len() == 0, "vec IntoIter default", 0);

    // From<&mut [T]>, From<&[T; N]>, From<&mut [T; N]>.
    let mut arr = [1, 2];
    run.check(alloc::vec::Vec::<i32>::from(&mut arr[..]) == [1, 2], "vec from slices", 0);
    run.check(alloc::vec::Vec::<i32>::from(&[1, 2]) == [1, 2], "vec from slices", 1);
    run.check(alloc::vec::Vec::<i32>::from(&mut arr) == [1, 2], "vec from slices", 2);

    // String::shrink_to.
    let mut st = alloc::string::String::with_capacity(16);
    st.push_str("ab");
    st.shrink_to(4);
    run.check(st.capacity() == 4 && st.as_str() == "ab", "string shrink_to", 0);
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let mut run = Run { passed: 0, failed: 0 };
    // Few distinct keys (constant churn on one or two levels), mid, and wide (four levels deep).
    session(&mut run, 1, 30, 5_000);
    session(&mut run, 2, 50, 20_000);
    session(&mut run, 7, 2_000, 200_000);
    session(&mut run, 99, 100_000, 300_000);
    sets(&mut run);
    vecs_api(&mut run);
    btrees_api(&mut run);
    strings_api(&mut run);
    vecdeques(&mut run);
    arc_layouts(&mut run);
    heaps(&mut run);
    rcs(&mut run);
    sorts(&mut run);
    // Descending inserts: every one lands at the front, which the sorted Vec paid for with a full move.
    let mut map = BTreeMap::new();
    let mut k = 200_000u32;
    while k > 0 {
        k -= 1;
        map.insert(k, k as u64);
    }
    run.check(map.len() == 200_000 && map.keys().copied().eq(0..200_000), "descending inserts", 0);
    out("passed ");
    out_u64(run.passed);
    out(", failed ");
    out_u64(run.failed);
    out("\n");
    if run.failed == 0 { 0 } else { 1 }
}
