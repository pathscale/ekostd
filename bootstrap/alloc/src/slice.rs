impl<T> [T] {
    /// Sorts by a key computed once per element; equal keys keep their order.
    #[rustc_allow_incoherent_impl]
    pub fn sort_by_cached_key<K, F>(&mut self, mut f: F) where F: FnMut(&T) -> K, K: Ord {
        let len = self.len();
        if len < 2 { return; }
        let mut indices: crate::vec::Vec<(K, usize)> = crate::vec::Vec::with_capacity(len);
        for (i, x) in self.iter().enumerate() { indices.push((f(x), i)); }
        indices.sort();
        for i in 0..len {
            let mut index = indices[i].1;
            while index < i { index = indices[index].1; }
            indices[i].1 = index;
            self.swap(i, index);
        }
    }
    #[rustc_allow_incoherent_impl]
    pub fn to_vec(&self) -> crate::vec::Vec<T> where T: Clone {
        let mut result = crate::vec::Vec::with_capacity(self.len());
        result.extend_from_slice(self);
        result
    }
    #[rustc_allow_incoherent_impl]
    pub fn into_vec(self: crate::boxed::Box<[T]>) -> crate::vec::Vec<T> {
        crate::boxed::Box::into_vec(self)
    }
    #[rustc_allow_incoherent_impl]
    pub fn sort(&mut self) where T: Ord { self.sort_by(Ord::cmp); }
    #[rustc_allow_incoherent_impl]
    pub fn sort_unstable(&mut self) where T: Ord { self.sort_unstable_by(Ord::cmp); }
    /// A new vector of this slice repeated `n` times; panics if the length overflows.
    #[rustc_allow_incoherent_impl]
    pub fn repeat(&self, n: usize) -> crate::vec::Vec<T> where T: Copy {
        let capacity = self.len().checked_mul(n).expect("capacity overflow");
        let mut out = crate::vec::Vec::with_capacity(capacity);
        let mut i = 0;
        while i < n {
            out.extend_from_slice(self);
            i += 1;
        }
        out
    }
}

/// What `[V]::concat` joins into: a `String` for strings, a `Vec<T>` for slices.
pub trait Concat<Item: ?Sized> {
    type Output;
    fn concat(slice: &Self) -> Self::Output;
}
impl<T: Clone, V: core::borrow::Borrow<[T]>> Concat<T> for [V] {
    type Output = crate::vec::Vec<T>;
    fn concat(slice: &Self) -> crate::vec::Vec<T> {
        let mut size = 0;
        for v in slice.iter() { size += v.borrow().len(); }
        let mut result = crate::vec::Vec::with_capacity(size);
        for v in slice.iter() { result.extend_from_slice(v.borrow()); }
        result
    }
}
impl<S: core::borrow::Borrow<str>> Concat<str> for [S] {
    type Output = crate::string::String;
    fn concat(slice: &Self) -> crate::string::String {
        let mut out = crate::string::String::new();
        for piece in slice.iter() { out.push_str(piece.borrow()); }
        out
    }
}
impl<T> [T] {
    #[rustc_allow_incoherent_impl]
    pub fn concat<Item: ?Sized>(&self) -> <[T] as Concat<Item>>::Output where [T]: Concat<Item> {
        Concat::concat(self)
    }
}

/// What `[V]::join` builds: a `String` for strings with a `&str` separator, a `Vec<T>` for slices with a `&T` or
/// `&[T]` separator.
pub trait Join<Separator> {
    type Output;
    fn join(slice: &Self, sep: Separator) -> Self::Output;
}
impl<S: core::borrow::Borrow<str>> Join<&str> for [S] {
    type Output = crate::string::String;
    /// The strings one after another, with `sep` between each two.
    fn join(slice: &Self, sep: &str) -> crate::string::String {
        let mut out = crate::string::String::new();
        for (index, piece) in slice.iter().enumerate() {
            if index != 0 {
                out.push_str(sep);
            }
            out.push_str(piece.borrow());
        }
        out
    }
}
impl<T: Clone, V: core::borrow::Borrow<[T]>> Join<&T> for [V] {
    type Output = crate::vec::Vec<T>;
    /// The slices one after another, with a clone of `sep` between each two.
    fn join(slice: &Self, sep: &T) -> crate::vec::Vec<T> {
        let mut out = crate::vec::Vec::new();
        for (index, piece) in slice.iter().enumerate() {
            if index != 0 {
                out.push(sep.clone());
            }
            out.extend_from_slice(piece.borrow());
        }
        out
    }
}
impl<T: Clone, V: core::borrow::Borrow<[T]>> Join<&[T]> for [V] {
    type Output = crate::vec::Vec<T>;
    /// The slices one after another, with the elements of `sep` between each two.
    fn join(slice: &Self, sep: &[T]) -> crate::vec::Vec<T> {
        let mut out = crate::vec::Vec::new();
        for (index, piece) in slice.iter().enumerate() {
            if index != 0 {
                out.extend_from_slice(sep);
            }
            out.extend_from_slice(piece.borrow());
        }
        out
    }
}
impl<T> [T] {
    /// The elements one after another, with `sep` between each two; an empty slice joins to an empty result.
    #[rustc_allow_incoherent_impl]
    pub fn join<Separator>(&self, sep: Separator) -> <[T] as Join<Separator>>::Output where [T]: Join<Separator> {
        Join::join(self, sep)
    }
}
