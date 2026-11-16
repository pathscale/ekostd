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

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let mut run = Run { passed: 0, failed: 0 };
    // Few distinct keys (constant churn on one or two levels), mid, and wide (four levels deep).
    session(&mut run, 1, 30, 5_000);
    session(&mut run, 2, 50, 20_000);
    session(&mut run, 7, 2_000, 200_000);
    session(&mut run, 99, 100_000, 300_000);
    sets(&mut run);
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
