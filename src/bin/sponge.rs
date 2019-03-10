use std::env;
use std::ffi;
use std::fs;
use std::io;
use std::io::Seek;
use std::io::SeekFrom;
use std::path::Path;
use std::path::PathBuf;
use std::process;

fn run() -> u8 {
    let mut args = env::args_os();
    let us = args
        .next()
        .unwrap_or_else(ffi::OsString::new)
        .into_string()
        .unwrap_or_else(|_| String::new());
    let short_usage = format!("Usage: {} [-a] FILE", us);

    let mut opts = getopts::Options::new();
    opts.optflag("a", "append", "append to output file");
    opts.optflag("h", "help", "print this help");
    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", opts.usage(&short_usage));
            eprintln!("Error: argument parsing failed: {:?}", e);
            return 3;
        }
    };

    if matches.opt_present("h") {
        println!("{}", opts.usage(&short_usage));
        return 0;
    }

    if 1 != matches.free.len() {
        eprintln!("{}", opts.usage(&short_usage));
        eprintln!("Error: expected exactly one FILE");
        return 4;
    }

    let append = matches.opt_present("a");
    let output = PathBuf::from(matches.free.into_iter().next().unwrap());

    let mut parent = output.parent().unwrap_or_else(|| Path::new(""));
    if parent.as_os_str().is_empty() {
        parent = Path::new(".");
    }
    let mut tmp = match tempfile_fast::PersistableTempFile::new_in(parent) {
        Ok(tmp) => tmp,
        Err(e) => {
            eprintln!("Error: unable to create temporary file: {}", e);
            return 5;
        }
    };

    if let Err(e) = io::copy(&mut io::stdin().lock(), &mut tmp) {
        eprintln!("Error: temp file writing failed: {}", e);
        return 6;
    }

    let mut tmp = match tmp.persist_noclobber(&output) {
        Ok(()) => return 0,
        Err(e) => e.file,
    };

    if append {
        if let Err(e) = tmp.seek(SeekFrom::Start(0)) {
            eprintln!("Error: couldn't rewind temp file: {}", e);
            return 7;
        }
        let mut output_file = match fs::OpenOptions::new().append(true).write(true).open(output) {
            Ok(output_file) => output_file,
            Err(e) => {
                eprintln!("Error: opening output file for append: {}", e);
                return 8;
            }
        };

        if let Err(e) = io::copy(&mut tmp, &mut output_file) {
            eprintln!("Error: writing output: {}", e);
            return 9;
        }
    } else {
        if let Err(e) = tmp.persist_by_rename(output) {
            eprintln!("Error: overwriting output failed: {}", e.error);
            return 10;
        }
    }

    0
}

fn main() {
    process::exit(i32::from(run()));
}
