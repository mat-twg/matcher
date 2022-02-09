extern crate crossbeam;
extern crate crossbeam_channel;
extern crate csv;
extern crate lazy_static;

use std::collections::HashMap;
use std::io::{self, BufRead, Error, Write};
use std::process;
use std::sync::Mutex;

use crossbeam_channel::unbounded;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref REGEX_MAP: Mutex<HashMap<String, Regex>> = Mutex::new(HashMap::new());
}

fn read(chunk: (usize, &[Result<String, Error>])) -> Result<(usize, Vec<String>), Box<dyn std::error::Error>> {
    let mut chunk_buff: Vec<String> = Vec::new();

    for line in chunk.1.iter()
    {
        chunk_buff.push(line.as_ref().unwrap().to_string())
    }
    let buff = chunk_buff.join("\n");
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_reader(buff.as_bytes());

    let mut out = Vec::new();
    let mut regex_map_clone = REGEX_MAP.lock().unwrap().clone();

    for result in rdr.records() {
        let record = result?;
        if !regex_map_clone.contains_key(&record[1]) {
            REGEX_MAP.lock().unwrap().insert(record[1].to_string(), Regex::new(&format!(r"{}", record[1].to_string())).unwrap());
            regex_map_clone = REGEX_MAP.lock().unwrap().clone();
        }
        let regex = &regex_map_clone[&record[1]];
        let is_match = regex.is_match(&record[0]) as i8;
        out.push(is_match.to_string());
    }
    Ok((chunk.0, out))
}

fn main() {
    let stdin = io::stdin();
    let lines = stdin.lock().lines();
    let lines_raw: Vec<_> = lines.collect();
    let chunk_lines_size: usize = 1000;
    let mut out_buff = vec![String::from(""); chunk_lines_size];

    let n_workers = num_cpus::get();

    let (snd1, rcv1) = unbounded();
    let (snd2, rcv2) = unbounded();

    crossbeam::scope(|s| {
        // Producer thread
        s.spawn(|_| {
            for chunk in lines_raw.chunks(chunk_lines_size).enumerate() {
                snd1.send(chunk).unwrap();
            }
            drop(snd1);
        });
        for _ in 0..n_workers {
            let (snd, rcv) = (snd2.clone(), rcv1.clone());
            // Spawn workers in separate threads
            s.spawn(move |_| {
                // Receive until channel closes
                for chunk in rcv.iter() {
                    match read(chunk) {
                        Ok((chunk_num, chunk_lines)) => snd.send((chunk_num, chunk_lines)).unwrap(),
                        Err(err) => {
                            print!("Unexpected error: {}", err);
                            process::exit(1);
                        }
                    };
                }
            });
        }
        drop(snd2);
        // Sink
        for (chunk_num, chunk_results) in rcv2.iter() {
            out_buff[chunk_num] = chunk_results.join("\n");
        }
    }).unwrap();

    let mut out = Vec::new();
    for _out in out_buff {
        if _out.len() == 0 {
            continue;
        }
        out.push(_out);
    }
    print!("{}", out.join("\n"));
    io::stdout().flush().expect("Unable to flush stdout");
}
