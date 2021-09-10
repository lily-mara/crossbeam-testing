use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    os::linux::fs::MetadataExt,
    time::Instant,
};

fn run_thread_count(threads: usize) {
    let (line_tx, line_rx) = crossbeam_channel::bounded(1_000);

    std::thread::Builder::new()
        .name("ctest-read".into())
        .spawn(move || {
            let f = BufReader::new(File::open("data.txt").unwrap());

            for line in f.lines() {
                let line = line.unwrap();
                line_tx.send(line).unwrap();
            }
        })
        .unwrap();

    let (result_tx, result_rx) = crossbeam_channel::bounded(threads);

    for i in 0..threads {
        let line_rx = line_rx.clone();
        let done_tx = result_tx.clone();

        std::thread::Builder::new()
            .name(format!("ctest-wrk-{}", i))
            .spawn(move || {
                let mut count = 0;

                for x in line_rx {
                    for c in x.chars() {
                        if c == '9' {
                            count += 1;
                        }
                    }
                }

                done_tx.send(count).unwrap();
            })
            .unwrap();
    }

    drop(result_tx);

    let mut count = 0;
    for c in result_rx {
        count += c;
    }

    drop(count);
}

fn main() {
    let mut f = BufWriter::new(File::create("data.txt").unwrap());
    for i in 0..10_000_000 {
        writeln!(f, "line {}", i).unwrap();
    }
    f.flush().unwrap();
    drop(f);

    let size = File::open("data.txt")
        .unwrap()
        .metadata()
        .unwrap()
        .st_size();

    println!("Using {} MB of data\n", size / 1000 / 1000);

    for i in 1..=num_cpus::get_physical() {
        let start = Instant::now();
        run_thread_count(i);
        let elapsed = start.elapsed();

        let throughput = ((size as f64) / (elapsed.as_millis() as f64 / 1000.0)) / 1000.0 / 1000.0;

        println!(
            "{} threads - {:.2}s - {:.2} MB/s",
            i,
            elapsed.as_millis() as f64 / 1000.0,
            throughput,
        );
    }

    std::fs::remove_file("data.txt").unwrap();
}
