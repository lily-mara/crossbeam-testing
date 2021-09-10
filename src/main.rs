use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read, Write},
    os::linux::fs::MetadataExt,
    time::{Duration, Instant},
};

fn run_thread_count(threads: usize, mut lines: impl 'static + Send + Iterator<Item = String>) {
    let (line_tx, line_rx) = crossbeam_channel::bounded(1_000);

    std::thread::Builder::new()
        .name("ctest-read".into())
        .spawn(move || {
            let mut io_time = Duration::from_secs(0);
            let mut send_time = Duration::from_secs(0);

            loop {
                let start = Instant::now();
                let line = match lines.next() {
                    Some(line) => line,
                    None => break,
                };
                io_time += start.elapsed();

                let start = Instant::now();
                line_tx.send(line).unwrap();
                send_time += start.elapsed();
            }

            println!(
                "io: {:.2}s, send: {:.2}s",
                io_time.as_millis() as f64 / 1000.0,
                send_time.as_millis() as f64 / 1000.0,
            )
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

fn run_all<F, I>(size: u64, f: F)
where
    F: Fn() -> I,
    I: 'static + Send + Iterator<Item = String>,
{
    for i in 1..=num_cpus::get_physical() {
        let start = Instant::now();

        run_thread_count(i, f());
        let elapsed = start.elapsed();

        let throughput = ((size as f64) / (elapsed.as_millis() as f64 / 1000.0)) / 1000.0 / 1000.0;

        println!(
            "{} threads - {:.2}s - {:.2} MB/s\n",
            i,
            elapsed.as_millis() as f64 / 1000.0,
            throughput,
        );
    }
}

fn main() {
    let mut f = BufWriter::new(File::create("data.txt").unwrap());
    for i in 0..1_000_000 {
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

    println!("with file IO");
    run_all(size, || {
        BufReader::new(File::open("data.txt").unwrap())
            .lines()
            .map(|l| l.unwrap())
    });

    println!("\n\nwith in-memory data");
    run_all(size, || {
        let mut f = File::open("data.txt").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();

        s.lines()
            .map(|l| l.to_owned())
            .collect::<Vec<_>>()
            .into_iter()
    });

    std::fs::remove_file("data.txt").unwrap();
}
