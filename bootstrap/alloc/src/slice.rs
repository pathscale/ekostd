impl<T> [T] {
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
}

impl<S: core::borrow::Borrow<str>> [S] {
    /// The strings one after another, with `separator` between each two.
    #[rustc_allow_incoherent_impl]
    pub fn join(&self, separator: &str) -> crate::string::String {
        let mut out = crate::string::String::new();
        for (index, piece) in self.iter().enumerate() {
            if index != 0 {
                out.push_str(separator);
            }
            out.push_str(piece.borrow());
        }
        out
    }
}
