use chrono::{Local};
use env_logger::{Builder, Env, Target};
use std::fs::File;
use std::io::Write;
use std::thread;

pub fn init(level: &str, file_path: &Option<String>) {
    let mut builder = Builder::from_env(Env::default().default_filter_or(level));
    builder.format(|buffer, record| {
        writeln!(
            buffer,
            "{} [{:?}] {} [{}] - {}",
            Local::now().to_rfc3339(),
            thread::current().id(),
            buffer.default_styled_level(record.level()),
            record.target(),
            record.args()
        )
    });

    if let Some(file_path) = file_path {
        if let Ok(file) = File::create(file_path) {
            builder.target(Target::Pipe(Box::new(file)));
        }
    }

    builder.init();
}
