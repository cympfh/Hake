#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Total<T>(pub T);
impl<T: PartialEq> Eq for Total<T> {}
impl<T: PartialOrd> Ord for Total<T> {
    fn cmp(&self, rhs: &Total<T>) -> std::cmp::Ordering {
        self.0.partial_cmp(&rhs.0).unwrap()
    }
}
