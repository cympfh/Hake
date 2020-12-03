use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;

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
use metric::{average, Metric};
mod name;
mod options;
use options::*;
mod util;
use util::{sample, Total};
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
            // Brute-force
            let name = Arc::new(name);
            let args = Arc::new(args);
            let mut handles = VecDeque::new();
            for (id, param) in map.iter().enumerate() {
                let name = name.clone();
                let args = args.clone();
                let handle = thread::spawn(move || {
                    testone(&name, id, &args, &param, None);
                });
                handles.push_back(handle);
                while handles.len() >= opt.parallels() {
                    let handle_top = handles.pop_front().unwrap();
                    handle_top.join().unwrap();
                }
            }
            // Wait Rest All
            while let Some(handle) = handles.pop_front() {
                handle.join().unwrap();
            }
        }
        Some((obj, metric_name)) => {
            // Optimize by Differential Evolution
            eprintln!("{:?} `{}`", obj, &metric_name);
            let name = Arc::new(name);
            let args = Arc::new(args);
            let metric_name = Arc::new(metric_name);
            // DE vars
            let pool: Arc<Mutex<Vec<(Param, Metric)>>> = Arc::new(Mutex::new(vec![]));
            let id = Arc::new(Mutex::new(0));
            let job_queue = Arc::new(Mutex::new(VecDeque::new()));

            for gen in 0..opt.optimize.num_loop + 1 {
                if opt.debug || opt.verbose {
                    eprintln!("# Generation: {}", gen);
                }
                // Set Jobs Queue
                let poolsize = pool.lock().unwrap().len();
                if poolsize == 0 {
                    // Random Seeds to Fill Pool
                    for _ in 0..opt.optimize.np {
                        let param = map.rand();
                        if opt.debug {
                            eprintln!("Random Param: {:?}", &param);
                        }
                        let mut q = job_queue.lock().unwrap();
                        q.push_back(param);
                    }
                } else {
                    let pool = pool.lock().unwrap();
                    let mut q = job_queue.lock().unwrap();
                    // Evolution
                    for (x, _) in pool.iter() {
                        let a;
                        let b;
                        let c;
                        {
                            let indices = sample(&pool, 3);
                            let i = indices[0];
                            let j = indices[1];
                            let k = indices[2];
                            a = &pool[i].0;
                            b = &pool[j].0;
                            c = &pool[k].0;
                        }
                        let z = cross(&x, &a, &b, &c, &map, opt.optimize.cr, opt.optimize.factor);
                        if opt.debug {
                            eprintln!("DE: {:?} + ({:?}, {:?}, {:?}) => {:?}", &x, &a, &b, &c, &z);
                        }
                        q.push_back(z);
                    }
                }

                let mut handles = VecDeque::new();

                // Parallel Max to -j
                loop {
                    let next_job = job_queue.lock().unwrap().pop_front();
                    if let Some(param) = next_job {
                        let name = name.clone();
                        let args = args.clone();
                        let metric_name = metric_name.clone();
                        let metric_num_samples = opt.metric_num_samples().clone();
                        let pool = pool.clone();
                        let id = id.clone();
                        let handle = thread::spawn(move || {
                            let hid: usize;
                            {
                                let mut id = id.lock().unwrap();
                                hid = (*id).clone();
                                *id += 1;
                            }
                            let metric_samples: Vec<Metric> = (0..metric_num_samples)
                                .map(|_| testone(&name, hid, &args, &param, Some(&metric_name)))
                                .filter_map(|result| result)
                                .collect();
                            if let Some(result) = average(metric_samples) {
                                let mut pool = pool.lock().unwrap();
                                pool.push((param, result));
                            } else {
                                eprintln!("[Warning!] No Metric Report detected!");
                            }
                        });
                        handles.push_back(handle);
                    } else {
                        if opt.debug {
                            eprintln!("No More Job");
                        }
                        break;
                    }
                    while handles.len() >= opt.parallels() {
                        let handle_top = handles.pop_front().unwrap();
                        handle_top.join().unwrap();
                    }
                }
                // Run Rest All
                while let Some(handle) = handles.pop_front() {
                    handle.join().unwrap()
                }

                // Eliminate Top Seeds
                {
                    let mut pool = pool.lock().unwrap();
                    pool.sort_by_key(|item| Total(item.1.value));
                    if obj == Objective::Maximize {
                        pool.reverse();
                    }
                    pool.truncate(opt.optimize.np);
                    if opt.debug || opt.debug {
                        eprintln!("The {}-th Generation Top: {:?}", gen, pool[0]);
                    }
                }
            }

            // Finish
            let pool = pool.lock().unwrap();
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
