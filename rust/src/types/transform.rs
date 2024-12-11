pub(crate) trait MaybeFrom<T>: Sized {
    fn maybe_from(value: T) -> Option<Self>;
}

pub(crate) trait MaybeInto<T>: Sized {
    fn maybe_into(self) -> Option<T>;
}

impl<T, S> MaybeInto<S> for T
where
    S: MaybeFrom<T>,
{
    fn maybe_into(self) -> Option<S> {
        S::maybe_from(self)
    }
}
