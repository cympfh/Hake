/// Differential Evolution
extern crate rand;
use rand::distributions::{Distribution, Uniform};

use crate::map::{Map, Param, Value};

fn clip<T: PartialOrd>(x: T, min: T, max: T) -> T {
    let x = if x < min { min } else { x };
    if x > max {
        max
    } else {
        x
    }
}

pub fn cross(x: &Param, a: &Param, b: &Param, c: &Param, map: &Map, cr: f64, factor: f64) -> Param {
    let mut rng = rand::thread_rng();

    use Value::*;

    let cross_index = Uniform::from(0..map.len()).sample(&mut rng);
    let cross_prob = Uniform::new(0.0, 1.0);

    map.data
        .iter()
        .enumerate()
        .map(|(i, val)| {
            if i != cross_index && cross_prob.sample(&mut rng) > cr {
                x[i].clone()
            } else {
                let key = val.0.clone();
                match val.1 {
                    Val(_) | Int(_) | Float(_) => (key, val.1.clone()),
                    IntRange(begin, end, _) => match (&a[i].1, &b[i].1, &c[i].1) {
                        (Int(a), Int(b), Int(c)) => {
                            let z = (a.clone() as f64 + (b - c) as f64 * factor).round() as i64;
                            (key, Int(clip(z, begin, end)))
                        }
                        _ => panic!(),
                    },
                    FloatRange(begin, end, _) => match (&a[i].1, &b[i].1, &c[i].1) {
                        (Float(a), Float(b), Float(c)) => {
                            let z = a + (b - c) * factor;
                            (key, Float(clip(z, begin, end)))
                        }
                        _ => panic!(),
                    },
                    Choice(_) => a[i].clone(),
                }
            }
        })
        .collect()
}
