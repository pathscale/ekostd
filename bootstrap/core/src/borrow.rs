pub trait Borrow<Borrowed: ?Sized> {
    fn borrow(&self) -> &Borrowed;
}
pub trait BorrowMut<Borrowed: ?Sized>: Borrow<Borrowed> {
    fn borrow_mut(&mut self) -> &mut Borrowed;
}

impl<T: ?Sized> Borrow<T> for T {
    fn borrow(&self) -> &T { self }
}
impl<T: ?Sized> BorrowMut<T> for T {
    fn borrow_mut(&mut self) -> &mut T { self }
}
impl<T: ?Sized> Borrow<T> for &T {
    fn borrow(&self) -> &T { self }
}
impl<T: ?Sized> Borrow<T> for &mut T {
    fn borrow(&self) -> &T { self }
}
impl<T: ?Sized> BorrowMut<T> for &mut T {
    fn borrow_mut(&mut self) -> &mut T { self }
}
impl<T, const N: usize> Borrow<[T]> for [T; N] {
    fn borrow(&self) -> &[T] { self }
}
impl<T, const N: usize> BorrowMut<[T]> for [T; N] {
    fn borrow_mut(&mut self) -> &mut [T] { self }
}
