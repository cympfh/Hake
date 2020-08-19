extern crate rand;
use rand::distributions::{Distribution, Uniform};
use std::collections::BTreeSet;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Total<T>(pub T);
impl<T: PartialEq> Eq for Total<T> {}
impl<T: PartialOrd> Ord for Total<T> {
    fn cmp(&self, rhs: &Total<T>) -> std::cmp::Ordering {
        self.0.partial_cmp(&rhs.0).unwrap()
    }
}

fn choose<T>(xs: &Vec<T>, except: &BTreeSet<usize>) -> usize {
    let mut rng = rand::thread_rng();
    let indices = Uniform::from(0..xs.len());
    let mut idx = indices.sample(&mut rng);
    while except.contains(&idx) {
        idx = indices.sample(&mut rng);
    }
    idx
}

pub fn sample<T>(xs: &Vec<T>, n: usize) -> Vec<usize> {
    let mut r = vec![];
    let mut except = BTreeSet::new();
    while r.len() < n {
        let i = choose(&xs, &except);
        r.push(i);
        except.insert(i);
    }
    r
}
