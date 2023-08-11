use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader},
    process::{exit, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use chrono::{offset::Local, DateTime, Duration};
use cron::OwnedScheduleIterator;

struct Job {
    id: usize,
    upcoming: OwnedScheduleIterator<Local>,
    next: Option<DateTime<Local>>,
    command: String,
    is_running: bool,
}

type JobHandle = Arc<Mutex<Job>>;

fn main() {
    let args = ::std::env::args_os();
    if args.len() < 2 {
        eprintln!("Usage: pocketcron <crontab...>");
        exit(1);
    }

    let mut jobs = Vec::new();
    for crontab in args.skip(1) {
        load_jobs(&mut jobs, crontab);
    }

    loop {
        let now = Local::now();

        // Max sleep is 1 minute, to account for any clock jumps.
        let mut next_min = now + Duration::minutes(1);
        for job_handle in &jobs {
            let mut job = job_handle.lock().unwrap();

            let Some(next) = job.next else {
                continue;
            };

            if now < next {
                next_min = next.min(next_min);
                continue;
            }

            run_job(job_handle.clone());

            while job.next.filter(|next| now >= *next).is_some() {
                job.next = job.upcoming.next();
            }
        }

        if let Ok(delay) = (next_min - now).to_std() {
            thread::sleep(delay);
        }
    }
}

fn load_jobs(jobs: &mut Vec<JobHandle>, path: OsString) {
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(err) => {
            eprintln!("{}: open failed: {}", path.to_string_lossy(), err);
            exit(1);
        }
    };

    let now = Local::now();
    for (line_no, line) in BufReader::new(file).lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("{}: read failed: {}", path.to_string_lossy(), err);
                exit(1);
            }
        };

        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Use `str::split_whitespace` only to find the end of the schedule. We don't want to split the
        // command that way, because it could break spaces in quoted strings. Would prefer using
        // `SplitWhitespace::remainder`, but that is nightly-only at the moment.
        let command_start = if line.starts_with('@') {
            line.split_whitespace().nth(1)
        } else {
            line.split_whitespace().nth(5)
        };
        let Some(command_start) = command_start else {
            eprintln!("{}:{}: error: not enough elements", path.to_string_lossy(), line_no);
            exit(1);
        };
        let command_start = command_start.as_ptr() as usize - line.as_ptr() as usize;

        let schedule = &line[..command_start];
        let schedule = if schedule.starts_with('@') {
            schedule.to_owned()
        } else {
            // 'cron'-crate expects additional second and year elements.
            format!("0 {} *", schedule)
        };
        let schedule: ::cron::Schedule = match schedule.parse() {
            Ok(schedule) => schedule,
            Err(err) => {
                eprintln!("{}:{}: error: {}", path.to_string_lossy(), line_no, err);
                exit(1);
            }
        };

        let mut upcoming = schedule.after_owned(now);
        let next = upcoming.next();
        jobs.push(Arc::new(Mutex::new(Job {
            id: jobs.len() + 1,
            upcoming,
            next,
            command: line[command_start..].to_owned(),
            is_running: false,
        })));
    }
}

fn run_job(job_handle: JobHandle) {
    thread::spawn(move || {
        let (id, mut command) = {
            let mut job = job_handle.lock().unwrap();

            if job.is_running {
                return;
            }
            job.is_running = true;

            eprintln!("[{}] CMD {}", job.id, job.command);

            let mut command = Command::new("sh");
            command.arg("-c").arg(&job.command).stdin(Stdio::null());
            (job.id, command)
        };

        match command.spawn() {
            Err(err) => {
                eprintln!("[{}] spawn failed: {}", id, err);
            }
            Ok(mut proc) => match proc.wait() {
                Err(err) => {
                    eprintln!("[{}] wait failed: {}", id, err);
                }
                Ok(status) if !status.success() => {
                    eprintln!("[{}] {}", id, status);
                }
                _ => {}
            },
        };

        let mut job = job_handle.lock().unwrap();
        job.is_running = false;
    });
}
