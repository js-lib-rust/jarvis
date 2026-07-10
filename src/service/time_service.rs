use chrono::{Datelike, Duration, Local, NaiveDate};
use lazy_static::lazy_static;
use log::{debug, trace};
use regex::{Captures, Regex};

use crate::types::Result;
use crate::service::{Function, Property};

lazy_static! {
    // Get the date for today.
    static ref date_today_pattern: Regex = Regex::new(r"(?i)date .*today").unwrap();
    // Get the date for yesterday.
    static ref date_yesterday_pattern: Regex = Regex::new(r"(?i)date .*for yesterday").unwrap();
    // Get the date for the day before yesterday.
    static ref date_before_yesterday_pattern: Regex = Regex::new(r"(?i)date .*day before yesterday").unwrap();
    // Compute (age) as the difference in (years) between (2026-04-14) and (1964-03-15).
    static ref date_difference_pattern: Regex = Regex::new(r"(?i)^compute (.+) as the difference in (.+) between (\d{4}-\d{2}-\d{2}) and (\d{4}-\d{2}-\d{2}).$").unwrap();

    static ref functions: Vec<Function<'static>> = vec![
        Function {regex: &date_today_pattern, function: date_today},
        Function {regex: &date_yesterday_pattern, function: date_yesterday},
        Function {regex: &date_before_yesterday_pattern, function: date_before_yesterday},
        Function {regex: &date_difference_pattern, function: date_difference}
    ];
}

pub fn exec(prompt: &str) -> Result<String> {
    trace!("exec(prompt: &str) -> Result<String>");

    for f in functions.iter() {
        debug!("f: {:?}", f);
        if let Some(captures) = f.regex.captures(prompt) {
            debug!("captures: {:?}", captures);
            debug!("captures length: {}", captures.len());
            return (f.function)(&captures);
        }
    }

    Ok(String::new())
}

fn date_today(_captures: &Captures) -> Result<String> {
    trace!("date_today(captures: &Captures) -> Result<String>");
    let date = Local::now().date_naive();
    Ok(Property::value("date", &date.to_string()))
}

fn date_yesterday(_captures: &Captures) -> Result<String> {
    trace!("date_yesterday(captures: &Captures) -> Result<String>");
    let date = Local::now().date_naive() - Duration::days(1);
    Ok(Property::value("date", &date.to_string()))
}

fn date_before_yesterday(_captures: &Captures) -> Result<String> {
    trace!("date_before_yesterday(captures: &Captures) -> Result<String>");
    let date = Local::now().date_naive() - Duration::days(2);
    Ok(Property::value("date", &date.to_string()))
}

fn date_difference(captures: &Captures) -> Result<String> {
    trace!("date_difference(captures: &Captures) -> Result<String>");

    let variable = &captures[1];
    debug!("variable: {}", variable);
    let projection = &captures[2];
    debug!("projection: {}", projection);

    let date1 = captures[3].parse::<NaiveDate>()?;
    let date2 = captures[4].parse::<NaiveDate>()?;
    let (start, end) = if date1 <= date2 {
        (date1, date2)
    } else {
        (date2, date1)
    };
    debug!("start: {}", start);
    debug!("end: {}", end);

    let property = &format!("time duration in {projection}");
    let result = match projection {
        "years" => Property::value(property, &(end.year() - start.year()).to_string()),
        "months" => Property::value(property, &((end - start).num_days() / 30).to_string()),
        "weeks" => Property::value(property, &((end - start).num_weeks()).to_string()),
        "days" => Property::value(property, &((end - start).num_days()).to_string()),
        "hours" => Property::value(property, &((end - start).num_hours()).to_string()),
        "minutes" => Property::value(property, &((end - start).num_minutes()).to_string()),
        "seconds" => Property::value(property, &((end - start).num_seconds()).to_string()),
        _ => String::new(),
    };
    Ok(result)
}
