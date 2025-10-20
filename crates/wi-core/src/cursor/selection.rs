pub enum Element {}

pub trait Selection {
    fn union<I: IntoIterator<Item = Element>>(&mut self, it: I);

    fn difference<I: IntoIterator<Item = Element>>(&mut self, it: I);
}

impl<T: Selection> Selection for &mut T {
    #[inline]
    fn union<I: IntoIterator<Item = Element>>(&mut self, it: I) { T::union(self, it); }

    #[inline]
    fn difference<I: IntoIterator<Item = Element>>(&mut self, it: I) { T::difference(self, it); }
}

pub trait SelectionExt: Selection {
    #[inline]
    fn inverted_mut(&mut self) -> Invert<&mut Self> { Invert(self) }
}

impl<T: Selection> SelectionExt for T {}

pub struct NullSelection;

impl Selection for NullSelection {
    #[inline]
    fn union<I: IntoIterator<Item = Element>>(&mut self, _: I) {}

    #[inline]
    fn difference<I: IntoIterator<Item = Element>>(&mut self, _: I) {}
}

pub struct Invert<T>(T);

impl<T: Selection> Selection for Invert<T> {
    #[inline]
    fn union<I: IntoIterator<Item = Element>>(&mut self, it: I) { self.0.difference(it); }

    #[inline]
    fn difference<I: IntoIterator<Item = Element>>(&mut self, it: I) { self.0.union(it); }
}
