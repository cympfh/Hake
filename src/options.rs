use std::path::Path;

extern crate structopt;
use structopt::StructOpt;

use crate::map::*;
use crate::name;
use nom::{
    branch::alt, bytes::complete::tag, character::complete::digit1, combinator::map,
    sequence::terminated, IResult,
};

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(short, long, help = "For Developers")]
    pub debug: bool,

    #[structopt(short, long, help = "With Noisy Logging")]
    pub verbose: bool,

    #[structopt(short, long, help = "As H(M)akefile")]
    pub file: Option<String>,

    #[structopt(short, help = "num of threads for parallel", default_value = "1")]
    pub j: usize,

    #[structopt(long, help = "Experiment Name")]
    pub name: Option<String>,

    #[structopt(long, value_name = "metric", help = "Metric to Maximize", conflicts_with_all(&["min"]))]
    pub max: Option<String>,

    #[structopt(long, value_name = "metric", help = "Metric to Minimize")]
    pub min: Option<String>,

    #[structopt(
        long,
        short = "M",
        help = "Num of Samples for averaging metric",
        default_value = "1"
    )]
    pub metric_num_samples: usize,

    #[structopt(help = "Target in H(M)akefile")]
    pub target: Option<String>,

    #[structopt(name = "mapping", help = "KEY=VALUE or KEY=RANGE")]
    pub map: Vec<String>,

    #[structopt(flatten)]
    pub optimize: OptimizeOptions,

    #[structopt(
        short,
        long,
        parse(from_str = parse_timeout),
        help = "timeout sec (0 for no timeout) (e.g. --timeout 10, --timeout 30m, --timeout 1h)",
        default_value = "0"
    )]
    pub timeout: u64,
}

#[derive(Debug, StructOpt)]
pub struct OptimizeOptions {
    #[structopt(
        short = "-N",
        long,
        default_value = "40",
        help = "[Optimize] Num of Population"
    )]
    pub np: usize,

    #[structopt(
        short,
        long,
        default_value = "0.5",
        help = "[Optimize] Prob of Cross-Over"
    )]
    pub cr: f64,

    #[structopt(
        short = "-F",
        long,
        default_value = "0.5",
        help = "[Optimize] Inner Factor for Cross-Over"
    )]
    pub factor: f64,

    #[structopt(
        short = "L",
        long = "loop",
        default_value = "10",
        help = "[Optimize] Num of Loop"
    )]
    pub num_loop: usize,
}

impl Options {
    pub fn from() -> Self {
        Options::from_args()
    }

    /// -f or `Hakefile` or `Makefile`
    pub fn makefile(&self) -> Result<String, String> {
        if let Some(user_file) = &self.file {
            if !Path::new(&user_file).exists() {
                Err(format!("File `{}` Not Found", &user_file))
            } else {
                Ok(user_file.to_string())
            }
        } else {
            let makefiles = ["Hakefile", "Makefile"];
            for &f in makefiles.iter() {
                if Path::new(f).exists() {
                    return Ok(f.to_string());
                }
            }
            Err(format!("Not found Hakefile nor Makefile"))
        }
    }

    /// --name or auto-generated name
    pub fn name(&self) -> Result<String, String> {
        if let Some(name) = self.name.clone() {
            if name::exists(&name) {
                Err(format!("Name Already Exists: {}", name))
            } else {
                Ok(name)
            }
        } else {
            let mut name = name::gen();
            for _ in 0..1000 {
                if name::exists(&name) {
                    name = name::gen();
                } else {
                    break;
                }
            }
            if name::exists(&name) {
                Err(format!("Name exhausted!? Please consider to clean."))
            } else {
                Ok(name)
            }
        }
    }

    pub fn target_map(&self) -> (Vec<String>, Map) {
        let mut target = vec![];
        let mut map = Map::new();
        let args: Vec<String> = self.target.iter().chain(self.map.iter()).cloned().collect();
        for arg in args {
            if let Ok((key, val)) = Map::parse_pair(&arg) {
                map.add(key, val);
            } else {
                target.push(arg.clone());
            }
        }
        (target, map)
    }

    pub fn metric(&self) -> Option<(Objective, String)> {
        match (self.max.clone(), self.min.clone()) {
            (Some(name), None) => Some((Objective::Maximize, name)),
            (None, Some(name)) => Some((Objective::Minimize, name)),
            _ => None,
        }
    }

    pub fn metric_num_samples(&self) -> usize {
        self.metric_num_samples
    }

    pub fn parallels(&self) -> usize {
        std::cmp::min(self.j, self.optimize.np)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Objective {
    Minimize,
    Maximize,
}

/// parser for --timeout
fn parse_timeout(input: &str) -> u64 {
    fn read_int(input: &str) -> IResult<&str, u64> {
        let (rest, digits) = digit1(input)?;
        let value = digits.parse::<u64>().ok().unwrap();
        Ok((rest, value))
    }
    let mut parser = alt((
        map(terminated(read_int, tag("s")), |value: u64| value),
        map(terminated(read_int, tag("m")), |value: u64| value * 60),
        map(terminated(read_int, tag("h")), |value: u64| value * 60 * 60),
        map(terminated(read_int, tag("d")), |value: u64| {
            value * 60 * 60 * 24
        }),
        read_int,
    ));
    let (rest, value) = parser(input).ok().unwrap();
    if !rest.is_empty() {
        panic!("Parsing error --timeout");
    }
    value
}
