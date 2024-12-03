pub trait FitIntoArr<T> {
    fn fit_into_arr<const S: usize>(self) -> [T; S];
}

impl FitIntoArr<u8> for Vec<u8> {
    fn fit_into_arr<const S: usize>(mut self) -> [u8; S] {
        if self.len() < S {
            let mut padded_vec = vec![0u8; S - self.len()];
            padded_vec.extend(self);

            self = padded_vec;
        } else if self.len() > S {
            self.truncate(S);
        }

        let mut arr = [0u8; S];
        arr.copy_from_slice(&self);

        arr
    }
}
