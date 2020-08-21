use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufReader, Write};

extern crate regex;
use regex::Regex;

extern crate structopt;
use structopt::StructOpt;

extern crate serde;
extern crate serde_json;
use serde::Deserialize;
use serde_json::json;

extern crate hake;
use hake::map::{Map, Value};
use hake::metric::Metric;

#[derive(Debug, StructOpt)]
struct Options {
    #[structopt(name = "mapping", help = "KEY=VALUE or KEY=RANGE")]
    pub map: Vec<String>,
}

impl Options {
    pub fn map(&self) -> Map {
        let mut map = Map::new();
        for arg in self.map.clone() {
            if let Ok((key, val)) = Map::parse_pair(&arg) {
                map.add(key, val);
            }
        }
        map
    }
}

#[derive(Debug, Clone)]
struct LogLine {
    datetime: String,
    content: LogEntity,
}

#[derive(Debug, Clone)]
enum LogEntity {
    Make(MakeArgs),
    Metric(Metric),
    Stuff,
}

#[derive(Debug, Clone, Deserialize)]
struct MakeArgs {
    name: String,
    git_hash: String,
    make_args: Vec<String>,
}

impl MakeArgs {
    pub fn map(&self) -> Map {
        let mut map = Map::new();
        for arg in self.make_args.iter() {
            if let Ok((key, val)) = Map::parse_pair(&arg) {
                map.add(key, val);
            }
        }
        map
    }
    pub fn json(&self) -> BTreeMap<String, serde_json::Value> {
        let mut map = BTreeMap::new();
        for arg in self.make_args.iter() {
            if let Ok((key, val)) = Map::parse_pair(&arg) {
                match val {
                    Value::Val(x) => {
                        if let Ok(x) = x.parse::<i64>() {
                            let _ = map.insert(key, json!(x));
                        } else if let Ok(x) = x.parse::<f64>() {
                            let _ = map.insert(key, json!(x));
                        } else {
                            let _ = map.insert(key, serde_json::Value::String(x));
                        }
                    }
                    Value::Int(x) => {
                        let _ = map.insert(key, json!(x));
                    }
                    Value::Float(x) => {
                        let _ = map.insert(key, json!(x));
                    }
                    _ => (),
                }
            }
        }
        map
    }
}

impl LogEntity {
    fn parse(line: &str) -> Self {
        if let Ok(make) = serde_json::from_str::<MakeArgs>(line) {
            Self::Make(make)
        } else if let Ok(metric) = serde_json::from_str::<Metric>(line) {
            Self::Metric(metric)
        } else {
            Self::Stuff
        }
    }
}

struct LogParser {
    pattern: Regex,
}

impl LogParser {
    fn new() -> Self {
        Self {
            pattern: Regex::new(r"^\[([^\]]*)\]\s+(.*)$").unwrap(),
        }
    }
    fn parse(&self, line: String) -> Option<LogLine> {
        if let Some(captures) = self.pattern.captures(&line) {
            match (captures.get(1), captures.get(2)) {
                (Some(datetime), Some(message)) => Some(LogLine {
                    datetime: datetime.as_str().to_string(),
                    content: LogEntity::parse(message.as_str()),
                }),
                _ => None,
            }
        } else {
            None
        }
    }
}

fn map_match(pattern: &Map, refmap: &Map) -> bool {
    use Value::*;
    for (key, val) in pattern.data.iter() {
        match val {
            Val(_) | Int(_) | Float(_) => {
                if Some(val) != refmap.get(key) {
                    return false;
                }
            }
            IntRange(begin, end, _) => match refmap.get(key) {
                Some(Int(x)) => {
                    if !(begin <= x && x <= end) {
                        return false;
                    }
                }
                Some(Val(x)) => {
                    if let Ok(x) = x.parse::<i64>() {
                        if !(begin <= &x && &x <= end) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            },
            FloatRange(begin, end, _) => match refmap.get(key) {
                Some(Float(x)) => {
                    if !(begin <= x && x <= end) {
                        return false;
                    }
                }
                Some(Val(x)) => {
                    if let Ok(x) = x.parse::<f64>() {
                        if !(begin <= &x && &x <= end) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                _ => return false,
            },
            Choice(xs) => match refmap.get(key) {
                Some(Val(z)) => {
                    let mut ok = false;
                    for x in xs {
                        if x == z {
                            ok = true;
                        }
                    }
                    if !ok {
                        return false;
                    }
                }
                _ => return false,
            },
        }
    }
    true
}

fn main() -> io::Result<()> {
    let opt = Options::from_args();
    let map = opt.map();

    let log_parser = LogParser::new();

    for entry in fs::read_dir(".hake/log/")? {
        let path = entry?.path();
        if path.is_file() {
            let file = File::open(&path)?;
            let reader = BufReader::new(file);

            let mut matched = false;

            let mut make_args = None;
            let mut datetime_begin = None;
            let mut datetime_end = None;
            let mut metrics = BTreeMap::new();

            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some(log) = log_parser.parse(line) {
                        if datetime_begin == None {
                            datetime_begin = Some(log.datetime.clone());
                        }
                        datetime_end = Some(log.datetime.clone());
                        match log.clone().content {
                            LogEntity::Make(args) => {
                                matched = map_match(&map, &args.map());
                                if !matched {
                                    break;
                                }
                                make_args = Some(args);
                            }
                            LogEntity::Metric(metric) => {
                                metrics.insert(metric.metric, metric.value);
                            }
                            _ => {}
                        }
                    }
                }
            }

            if !matched || make_args.is_none() {
                continue;
            }

            let result = json!({
                "name": make_args.clone().unwrap().name,
                "git_hash": make_args.clone().unwrap().git_hash,
                "params": make_args.clone().unwrap().json(),
                "log_file": &path,
                "datetime": {
                    "begin": datetime_begin,
                    "end": datetime_end,
                },
                "metrics": metrics,
            });
            let r = writeln!(&mut io::stdout(), "{}", result.to_string());
            if r.is_err() {
                std::process::exit(0);
            }
        }
    }

    Ok(())
}
