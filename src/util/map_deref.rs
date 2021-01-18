use std::ops::{Deref, DerefMut};

pub fn map_deref<T, U, DT, FTU>(
    source: DT,
    mapping: FTU,
) -> impl Deref<Target=U>
    where
        DT: Deref<Target=T>,
        FTU: Fn(&T) -> &U
{
    struct Holder<T, DT: Deref<Target=T>, FTU> { source: DT, mapping: FTU }
    impl<T, U, DT: Deref<Target=T>, FTU: Fn(&T) -> &U> Deref for Holder<T, DT, FTU> {
        type Target = U;
        fn deref(&self) -> &U {
            (self.mapping)(self.source.deref())
        }
    }
    Holder { source, mapping }
}

pub fn map_deref_mut<T, U, DT, FTU, MFTU>(
    source: DT,
    mapping: FTU,
    mapping_mut: MFTU,
) -> impl DerefMut<Target=U>
    where
        DT: DerefMut<Target=T>,
        FTU: Fn(&T) -> &U,
        MFTU: Fn(&mut T) -> &mut U,
{
    struct Holder<T, U, DT, FTU, MFTU> where DT: DerefMut<Target=T>, FTU: Fn(&T) -> &U, MFTU: Fn(&mut T) -> &mut U { source: DT, mapping: FTU, mapping_mut: MFTU }
    impl<T, U, DT: DerefMut<Target=T>, FTU: Fn(&T) -> &U, MFTU: Fn(&mut T) -> &mut U> Deref for Holder<T, U, DT, FTU, MFTU> {
        type Target = U;
        fn deref(&self) -> &U {
            (self.mapping)(self.source.deref())
        }
    }
    impl<T, U, DT: DerefMut<Target=T>, FTU: Fn(&T) -> &U, MFTU: Fn(&mut T) -> &mut U> DerefMut for Holder<T, U, DT, FTU, MFTU> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            (self.mapping_mut)(self.source.deref_mut())
        }
    }
    Holder { source, mapping, mapping_mut }
}
