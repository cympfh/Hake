use std::path::Path;

extern crate structopt;
use structopt::StructOpt;

use crate::map::*;
use crate::name;

#[derive(Debug, StructOpt)]
pub struct Options {
    #[structopt(short, long, help = "For developers")]
    pub debug: bool,

    #[structopt(short, long, help = "noisy logging")]
    pub verbose: bool,

    #[structopt(short, long, help = "As Makefile")]
    pub file: Option<String>,

    #[structopt(long, help = "Experiment Name")]
    pub name: Option<String>,

    #[structopt(help = "TARGET")]
    pub target: Option<String>,

    #[structopt(help = "KEY=VALUE or KEY=RANGE")]
    pub map: Vec<String>,
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
    pub fn name(&self) -> String {
        if let Some(a) = self.name.clone() {
            a
        } else {
            name::name()
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
}
