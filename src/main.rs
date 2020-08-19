extern crate serde;
extern crate serde_json;
use serde_json::json;

extern crate chrono;
use chrono::prelude::*;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

mod map;
use map::*;
mod metric;
use metric::Metric;
mod name;
mod options;
use options::*;
mod util;
use util::Total;
mod de;
use de::cross;

fn log_file_name(name: &String, id: usize) -> String {
    let now = Local::now();
    format!(".hake/log/{}_{}_{:08}", now.format("%Y%m%d"), name, id)
}

fn make(opt: &Options) -> Result<(), String> {
    let name = opt.name()?;
    let (targets, map) = opt.target_map();

    eprintln!("\x1b[33mName: {}\x1b[0m", &name);

    let mut args = vec![String::from("-f"), opt.makefile()?];
    for t in targets {
        args.push(t.clone());
    }
    args.push(format!("NAME={}", &name));

    name::touch(&name).expect("Cannot put name file.");

    match opt.metric() {
        None => {
            for (id, param) in map.iter().enumerate() {
                testone(&name, id, &args, &param, None);
            }
        }
        Some((obj, metric_name)) => {
            // Optimize by Differential Evolution
            eprintln!("{:?} `{}`", obj, &metric_name);
            let mut pool = vec![];
            let mut id: usize = 0;
            while pool.len() < opt.optimize.np * 2 {
                let param = map.rand();
                if opt.debug {
                    eprintln!("Random Param: {:?}", &param);
                }
                if let Some(result) = testone(&name, id, &args, &param, Some(&metric_name)) {
                    pool.push((param, result));
                    id += 1;
                } else {
                    eprintln!("[Warning!] No Metric Report detected!");
                    continue;
                }
            }
            {
                pool.sort_by_key(|item| Total(item.1.value));
                if obj == Objective::Maximize {
                    pool.reverse();
                }
                pool = pool[0..opt.optimize.np].to_vec();
            }
            for _ in 0..opt.optimize.num_loop {
                for i in 0..opt.optimize.np {
                    let z = cross(
                        &pool[i].0,
                        &map,
                        &pool,
                        opt.optimize.cr,
                        opt.optimize.factor,
                    );
                    if opt.debug {
                        eprintln!("DE: {:?} => {:?}", &pool[i].0, &z);
                    }
                    if let Some(result) = testone(&name, id, &args, &z, Some(&metric_name)) {
                        pool.push((z, result));
                        id += 1;
                    } else {
                        eprintln!("[Warning!] No Metric Report detected!");
                    }
                }
                let param = map.rand();
                if let Some(result) = testone(&name, id, &args, &param, Some(&metric_name)) {
                    pool.push((param, result));
                    id += 1;
                } else {
                    eprintln!("[Warning!] No Metric Report detected!");
                }
                pool.sort_by_key(|item| Total(item.1.value));
                if obj == Objective::Maximize {
                    pool.reverse();
                }
                pool = pool[0..opt.optimize.np].to_vec();
            }

            println!(
                "\x1b[31m{} {} = {} when {:?}\x1b[0m",
                if obj == Objective::Maximize {
                    "Max"
                } else {
                    "Min"
                },
                metric_name,
                pool[0].1.value,
                pool[0].0
            );
        }
    }

    Ok(())
}

fn testone(
    name: &String,
    id: usize,
    args: &Vec<String>,
    param: &Vec<(String, Value)>,
    watching_metric: Option<&String>,
) -> Option<Metric> {
    let mut args = args.clone();
    args.push(format!("HID={}", id));
    for (key, val) in param.iter() {
        let s = match val {
            Value::Val(x) => format!("{}={}", key, x),
            Value::Int(x) => format!("{}={}", key, x),
            Value::Float(x) => format!("{}={}", key, x),
            _ => panic!("Cannot stringify"),
        };
        args.push(s);
    }
    let log = log_file_name(&name, id);
    eprintln!(
        "\x1b[34mHake (NAME={}, ID={}, log=>{:?})\x1b[0m",
        &name, id, log
    );
    let mut child = Command::new("make")
        .args(&args)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Something Error to Make");
    listen(&mut child, &name, &log, &args, watching_metric)
}

fn git_hash() -> String {
    let result = Command::new("git")
        .args(&["log", "--pretty=format:%H", "-1"])
        .stdout(Stdio::piped())
        .output();
    match result {
        Ok(output) => output.stdout.iter().map(|&c| c as char).collect::<String>(),
        _ => String::new(),
    }
}

fn listen(
    child: &mut Child,
    name: &String,
    log: &String,
    args: &Vec<String>,
    watching_metric: Option<&String>,
) -> Option<Metric> {
    use std::fs::{create_dir_all, OpenOptions};
    create_dir_all(".hake/log").unwrap();
    let mut log = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(log)
        .unwrap();

    let mut tee = |msg: String, is_metric: bool| {
        let now = Local::now();
        let line = format!("[{:?}] {}\n", now, msg);
        let _ = log.write_all(line.as_bytes());
        if is_metric {
            println!("[{:?}] \x1b[31m{}\x1b[0m", now, msg);
        } else {
            print!("{}", line);
        }
    };

    tee(
        json!({"name": &name, "make_args": &args, "git_hash": git_hash()}).to_string(),
        false,
    );

    let mut last_metric = None;

    if let Some(out) = child.stdout.as_mut() {
        let reader = BufReader::new(out);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if let Ok(metric) = serde_json::from_str::<Metric>(&line) {
                        let is_watching = watching_metric.as_ref() == Some(&&metric.metric);
                        if is_watching {
                            last_metric = Some(metric);
                        }
                        tee(line, is_watching);
                    } else {
                        tee(line, false);
                    }
                }
                _ => {}
            }
        }
    }

    last_metric
}

fn main() -> Result<(), String> {
    let opt = Options::from();
    if opt.debug {
        eprintln!("{:?}", &opt);
    }
    make(&opt)
}
