pub trait VecExt<T: Clone> {
    fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T>;
}

impl<T: Clone> VecExt<T> for Vec<T> {
    fn query_items(&self, start: usize, end: Option<usize>) -> Vec<T> {
        let end = end.unwrap_or(self.len());
        if start >= self.len() || end <= start {
            vec![]
        } else {
            self[start..end].to_vec()
        }
    }
}
