use std::process::Command;

mod map;
mod name;
use map::*;

mod options;
use options::*;

fn make(opt: &Options) -> Result<(), String> {
    let (targets, map) = opt.target_map();
    let name = opt.name();
    eprintln!("Experiment Name: {}", &name);

    let mut args = vec![String::from("-f"), opt.makefile()?];
    for t in targets {
        args.push(t.clone());
    }
    args.push(format!("NAME={}", &name));

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
        if opt.debug {
            eprintln!("make id={} args={:?}", id, &args);
        }
        Command::new("make")
            .args(args)
            .spawn()
            .expect("Something Error to Make");
    }

    Ok(())
}

fn main() -> Result<(), String> {
    let opt = Options::from();
    if opt.debug || opt.verbose {
        eprintln!("{:?}", &opt);
    }
    make(&opt)
}
