extern crate rand;
use rand::distributions::{Distribution, Uniform};

pub type Param = Vec<(String, Value)>;

#[derive(Debug, Clone)]
pub struct Map {
    pub data: Param,
}

impl Map {
    pub fn new() -> Self {
        Map { data: Vec::new() }
    }
    pub fn parse_pair(pair: &String) -> Result<(String, Value), String> {
        let f = pair.split('=').collect::<Vec<_>>();
        if f.len() == 2 {
            let key = f[0].to_string();
            let value = Value::from(&f[1].to_string())?;
            Ok((key, value))
        } else {
            Err(format!("Parse Error: {:?}", &pair))
        }
    }
    pub fn add(&mut self, key: String, val: Value) {
        self.data.push((key, val));
    }
    pub fn iter(&self) -> MapIter {
        MapIter {
            idx: 0,
            data: self.clone(),
        }
    }
    pub fn index(&self, idx: usize) -> Param {
        let mut ret = vec![];
        let mut i = idx;
        for (key, val) in self.data.iter() {
            ret.push((key.clone(), val.index(i % val.len())));
            i /= val.len();
        }
        ret
    }
    pub fn len(&self) -> usize {
        let mut prod = 1;
        for (_, val) in self.data.iter() {
            prod *= val.len();
        }
        prod
    }
    pub fn rand(&self) -> Param {
        let mut rng = rand::thread_rng();
        let range = Uniform::from(0..self.len());
        let idx = range.sample(&mut rng);
        self.index(idx)
    }
}

pub struct MapIter {
    idx: usize,
    data: Map,
}

impl Iterator for MapIter {
    type Item = Vec<(String, Value)>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.data.len() {
            None
        } else {
            let ret = self.data.index(self.idx);
            self.idx += 1;
            Some(ret)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Val(String),
    Int(i64),
    Float(f64),
    Choice(Vec<String>),
    IntRange(i64, i64, i64), // begin, end, skip
    FloatRange(f64, f64, f64),
}

fn parse_number<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    if let Ok(num) = s.parse::<T>() {
        Ok(num)
    } else {
        Err(format!("Cannot parse as number: {:?}", s))
    }
}

impl Value {
    pub fn len(&self) -> usize {
        use Value::*;
        match self {
            Val(_) | Int(_) | Float(_) => 1,
            Choice(xs) => xs.len(),
            IntRange(begin, end, skip) => ((end - begin) / skip + 1) as usize,
            FloatRange(begin, end, skip) => ((end - begin) / skip + 1.0).floor() as usize,
        }
    }

    pub fn index(&self, i: usize) -> Self {
        use Value::*;
        match &self {
            Val(_) | Int(_) | Float(_) => self.clone(),
            Choice(xs) => Val(xs[i].clone()),
            IntRange(begin, _, skip) => Int(begin + skip * i as i64),
            FloatRange(begin, _, skip) => Float(begin + skip * i as f64),
        }
    }

    pub fn from(val: &String) -> Result<Self, String> {
        if val.contains("...") {
            let f = val.split("...").collect::<Vec<_>>();
            match f.len() {
                2 => {
                    let begin = parse_number::<f64>(f[0])?;
                    let end = parse_number::<f64>(f[1])?;
                    let skip = (end - begin) / 10.0;
                    if begin <= end {
                        Ok(Value::FloatRange(begin, end, skip))
                    } else {
                        Err(format!(
                            "Float-Range ... should be begin <= end: {:?}",
                            &val
                        ))
                    }
                }
                3 => {
                    let begin = parse_number::<f64>(f[0])?;
                    let second = parse_number::<f64>(f[1])?;
                    let end = parse_number::<f64>(f[2])?;
                    let skip = second - begin;
                    Ok(Value::FloatRange(begin, end, skip))
                }
                _ => Err("Float-Range ... should have 2 or 3 fields. See document.".to_string()),
            }
        } else if val.contains("..") {
            let f = val.split("..").collect::<Vec<_>>();
            match f.len() {
                2 => {
                    let begin = parse_number::<i64>(f[0])?;
                    let end = parse_number::<i64>(f[1])?;
                    if begin <= end {
                        Ok(Value::IntRange(begin, end, 1))
                    } else {
                        Err(format!("Int-Range .. should be begin <= end: {:?}", &val))
                    }
                }
                3 => {
                    let begin = parse_number::<i64>(f[0])?;
                    let second = parse_number::<i64>(f[1])?;
                    let end = parse_number::<i64>(f[2])?;
                    let skip = second - begin;
                    if (skip > 0 && begin <= end) || (skip < 0 && begin >= end) {
                        Ok(Value::IntRange(begin, end, skip))
                    } else {
                        Err(format!(
                            "Int-Range .. has strange second value...? {:?}",
                            &val
                        ))
                    }
                }
                _ => Err("Int-Range .. should have 2 or 3 fields. See document.".to_string()),
            }
        } else if val.contains(",") {
            Ok(Value::Choice(
                val.split(',').map(|s| s.to_string()).collect(),
            ))
        } else {
            Ok(Value::Val(val.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_from() {
        assert_eq!(
            Value::from(&String::from("1")),
            Ok(Value::Val(String::from("1")))
        );
        assert_eq!(
            Value::from(&String::from("1,2,3,banana")),
            Ok(Value::Choice(vec![
                String::from("1"),
                String::from("2"),
                String::from("3"),
                String::from("banana"),
            ]))
        );
        assert_eq!(
            Value::from(&String::from("1..3")),
            Ok(Value::IntRange(1, 3, 1))
        );
        assert_eq!(
            Value::from(&String::from("1..5..10")),
            Ok(Value::IntRange(1, 10, 4))
        );
        assert_eq!(
            Value::from(&String::from("0...10")),
            Ok(Value::FloatRange(0.0, 10.0, 1.0))
        );
        assert_eq!(
            Value::from(&String::from("0...2...10")),
            Ok(Value::FloatRange(0.0, 10.0, 2.0))
        );
        assert_eq!(
            Value::from(&String::from("0...-1...-5")),
            Ok(Value::FloatRange(0.0, -5.0, -1.0))
        );
    }

    #[test]
    fn value_len() {
        assert_eq!(
            Value::Choice(vec!["a".to_string(), "a".to_string()]).len(),
            2
        );
        assert_eq!(Value::IntRange(0, 10, 1).len(), 11);
        assert_eq!(Value::FloatRange(0.0, 10.0, 1.0).len(), 11);
        assert_eq!(Value::FloatRange(0.0, 10.0, 2.0).len(), 6);
    }

    #[test]
    fn map_parse_pair() {
        assert_eq!(
            Map::parse_pair(&String::from("KEY=VALUE")),
            Ok((String::from("KEY"), Value::Val(String::from("VALUE"))))
        );
        assert_eq!(
            Map::parse_pair(&String::from("favorite=apple,banana,lemon")),
            Ok((
                String::from("favorite"),
                Value::Choice(vec![
                    String::from("apple"),
                    String::from("banana"),
                    String::from("lemon"),
                ])
            ))
        );
    }
}
