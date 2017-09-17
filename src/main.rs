extern crate clap;
extern crate tempdir;

use clap::{App, Arg};
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Child};
use tempdir::TempDir;

// Idea, compare stdouts instead of exit values

fn do_run<T: AsRef<str>>(command: T, parallel: usize, parallel_min: usize) -> bool {
    let mut children: Vec<Child> = Vec::with_capacity(parallel);
    for _ in 0..parallel {
        children.push(
            Command::new("sh")
                .arg("-c")
                .arg(command.as_ref())
                .spawn()
                .expect("Failed to execute test"),
        );
    }

    let mut passed = 0usize;

    // NOTE: Fibbonacci sequence:
    // 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 100, 100...
    // because phi**x exponential backoff is slightly slower than 2**x
    let mut sleep1 = 1;
    let mut sleep2 = 1;

    while passed < parallel_min && !children.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(sleep1));
        let tmp = sleep1;
        sleep1 += sleep2;
        sleep2 = tmp;
        if sleep1 > 100 {
            sleep1 = 100
        }
        let tmp_children = std::mem::replace(&mut children, Vec::new());
        for mut child in tmp_children {
            if let Some(exit) = child.try_wait().unwrap() {
                // TODO: Interesting exit codes
                if exit.code().unwrap() == 0 {
                    passed += 1;
                };
            } else {
                children.push(child)
            }
        }
    }

    for mut child in children {
        child.kill().unwrap()
    }

    passed >= parallel_min
}

fn main() {
    let matches = App::new("reducewrap")
        .version("0.1")
        .author("Mateusz Naściszewski <matin1111@wp.pl>")
        .about("Useful helper for testcase reducer scripts")
        .arg(Arg::with_name("COMMAND").required(true).index(1).help(
            "Shell command to wrap. Replaces {} with tested filename, or appends as last argument",
        ))
        .arg(Arg::with_name("TESTCASE").required(true).index(2).help(
            "File being reduced",
        ))
        .arg(
            Arg::with_name("use_filename")
                .short("f")
                .long("filename")
                .takes_value(true)
                .default_value("tested")
                .value_name("FILENAME")
                .help("filename to use when testing"),
        )
        .arg(
            Arg::with_name("interesting_exits")
                .short("x")
                .long("interesting-exits")
                .takes_value(true)
                .value_name("EXIT_CODES")
                .conflicts_with("interesting_stdout")
                // Examples: !0; !1-3,127; 48-64
                .help("interesting exit values, comma separated list of ranges"),
        )
        .arg(
            Arg::with_name("interesting_stdout")
                .short("O")
                .long("interesting-out")
                .takes_value(true)
                .multiple(true)
                .min_values(1)
                .max_values(2)
                .value_names(&["CMP_STDOUT", "CMP_STDERR"])
                .conflicts_with("interesting_exits")
                .help("interesting stdout and (optional) stderr for comparison"),
        )
        .arg(
            Arg::with_name("validator")
                .short("v")
                .long("validator")
                .takes_value(true)
                .value_name("VALIDATOR_SCRIPT")
                .help("Shell command, skips longer tests if it exits non-zero"),
        )
        .arg(
            Arg::with_name("parallel")
                .short("P")
                .long("parallel")
                .takes_value(true)
                .default_value("1")
                .value_name("WORKERS")
                .help("number of tests to run in parallel"),
        )
        .arg(
            Arg::with_name("required_passes")
                .short("R")
                .long("required-passes")
                .takes_value(true)
                .default_value("1")
                .value_name("PASSES")
                .requires("parallel")
                .help("number of parallel tests required to pass"),
        )
        .arg(
            Arg::with_name("consistency")
                .short("C")
                .long("consistency")
                .takes_value(true)
                .default_value("1")
                .value_name("RUNS")
                .help("number of times to re-run all tests to ensure consistency"),
        )
        .get_matches();

    // First, parse args that can fail

    let runs: usize = match matches.value_of("consistency").unwrap().parse() {
        Err(e) => {
            println!("Expected consistency argument to be a positive integer");
            println!("{}", e);
            std::process::exit(1)
        }
        Ok(0) => {
            println!("Expected consistency argument to be a positive integer");
            std::process::exit(1)
        }
        Ok(v) => v,
    };

    let parallel: usize = match matches.value_of("parallel").unwrap().parse() {
        Err(e) => {
            println!("Expected parallel argument to be a positive integer");
            println!("{}", e);
            std::process::exit(1)
        }
        Ok(0) => {
            println!("Expected parallel argument to be a positive integer");
            std::process::exit(1)
        }
        Ok(v) => v,
    };

    let parallel_min: usize = match matches.value_of("required_passes").unwrap().parse() {
        Err(e) => {
            println!("Expected number of required passes to be a positive integer");
            println!("{}", e);
            std::process::exit(1)
        }
        Ok(0) => {
            println!("Expected number of required passes to be a positive integer");
            std::process::exit(1)
        }
        Ok(v) => {
            if v > parallel {
                println!(
                    "Expected number of required passes to be <= parallel passes ({})",
                    parallel
                );
                std::process::exit(1)
            }
            v
        }
    };

    let test_file = matches.value_of_os("use_filename").unwrap();

    let tmp = TempDir::new("reducewrap").expect("Couldn't create temp directory");
    let filename_path = {
        let tmp_path = tmp.path();
        tmp_path.join(test_file)
    };
    let filename = filename_path.to_str().unwrap();

    let testcase_arg = matches.value_of_os("TESTCASE").unwrap();

    {
        let mut tmp_file =
            File::create(filename_path.clone()).expect("Couldn't create temporary file");
        let mut orig_file = File::open(testcase_arg).unwrap_or_else(|err| {
            panic!("Couldn't open testcase file {:?}: {}", testcase_arg, err)
        });
        let mut testcase_bytes = Vec::new();
        orig_file.read_to_end(&mut testcase_bytes).expect(
            "Couldn't read from testcase file",
        );
        tmp_file.write_all(testcase_bytes.as_slice()).expect(
            "Couldn't write to tmp file",
        );
    }

    let validator = match matches.value_of("validator") {
        Some(v) => Some(if v.contains("{}") {
            v.replace("{}", filename)
        } else {
            v.to_string() + " " + filename
        }),
        None => None,
    };

    if let Some(v) = validator {
        println!("Running validator");
        let p = Command::new("sh")
            .arg("-c")
            .arg(v)
            .status()
            .unwrap()
            .code()
            .unwrap();
        if p != 0 {
            println!("Validator script exited {}, exiting", p);
            tmp.close().unwrap();
            std::process::exit(p);
        } else {
            println!("Success, test is valid");
        }
    }

    let command_arg = matches.value_of("COMMAND").unwrap().to_string();

    let command = if command_arg.contains("{}") {
        command_arg.replace("{}", filename)
    } else {
        command_arg + " " + filename
    };

    for run in 1..runs + 1 {
        println!("Run {}", run);
        if do_run(&command, parallel, parallel_min) {
            println!("Successful run {}/{}", run, runs);
        } else {
            println!("Failed run {}/{}", run, runs);
            tmp.close().unwrap();
            std::process::exit(1);
        }
    }

}
