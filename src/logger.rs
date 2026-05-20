use chrono::Local;
use env_logger::{Builder, Env, Target};
use std::fs::OpenOptions;
use std::io::Write;
use std::thread;

pub(crate) fn init(level: &str, file_path: &Option<String>) {
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
        let file_result = OpenOptions::new().append(true).create(true).open(file_path);
        if let Ok(file) = file_result {
            builder.target(Target::Pipe(Box::new(file)));
        } else {
            eprintln!("Failed to open log file: {:?}", file_path);
        }
    }

    builder.init();
}
