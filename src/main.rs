extern crate serde_json;
use serde_json::json;

extern crate chrono;
use chrono::prelude::*;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

mod map;
mod name;
use map::*;

mod options;
use options::*;

fn make(opt: &Options) -> Result<(), String> {
    let (targets, map) = opt.target_map();
    let name = opt.name();
    eprintln!("\x1b[33mExperiment Name: {}\x1b[0m", &name);

    let mut args = vec![String::from("-f"), opt.makefile()?];
    for t in targets {
        args.push(t.clone());
    }
    args.push(format!("NAME={}", &name));

    fn log_file_name(name: &String, id: usize) -> String {
        let now = Local::now();
        format!(".hake_log/{}_{}_{:08}", now.format("%Y%m%d"), name, id)
    }

    for (id, param) in map.iter().enumerate() {
        let mut args = args.clone();
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
        listen(&mut child, &name, &log, &args);
    }

    Ok(())
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

fn listen(child: &mut Child, name: &String, log: &String, args: &Vec<String>) {
    use std::fs::{create_dir_all, OpenOptions};
    create_dir_all(".hake_log").unwrap();
    let mut log = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(log)
        .unwrap();

    let mut tee = |line: String| {
        let now = Local::now();
        let line = format!("[{:?}] {}\n", now, line);
        let _ = log.write_all(line.as_bytes());
        print!("{}", line);
    };

    tee(json!({"name": &name, "make_args": &args, "git_hash": git_hash()}).to_string());

    if let Some(out) = child.stdout.as_mut() {
        let reader = BufReader::new(out);
        for line in reader.lines() {
            match line {
                Ok(line) => tee(line),
                _ => {}
            }
        }
    }
}

fn main() -> Result<(), String> {
    let opt = Options::from();
    if opt.debug || opt.verbose {
        eprintln!("{:?}", &opt);
    }
    make(&opt)
}
