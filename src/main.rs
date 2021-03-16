#[macro_use]
extern crate clap;
#[macro_use]
extern crate im;

use chrono::{Datelike, Local, NaiveDate};
use clap::{App, Arg};
use im::{HashSet, OrdSet};
use num::traits::FromPrimitive;
use regex::Regex;

fn main() -> Result<(), String> {
    let matches = App::new("cwver")
        .version(crate_version!())
        .author("Florian Bramer <elektronenhirn@gmail.com>")
        .about("Command line tool to work with calendar week version strings (e.g. 21w45.7).")
        .subcommand(
            App::new("today")
                .about("Display today's date as cw version string.")
        )
        .subcommand(
            App::new("convert")
                .about("Convert cw version string (e.g. 21w45.7) into ISO date.")
                .arg(
                    Arg::with_name("cw_ver_str")
                        .help("cw version string")
                        .index(1)
                        .required(true),
                )
        )
        .subcommand(
            App::new("bisect")
                .about("Calculates the workday(s) in the middle of two given cw versions spanning a regression range. Saturdays and sundays are ignored. Use --workdays to override.")
                .arg(
                    Arg::with_name("from")
                        .help("left side of the regression range")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("till")
                        .help("right side of the regression range")
                        .index(2)
                        .required(true),
                )
                .arg(
                    Arg::with_name("workdays")
                        .help("workdays")
                        .short("w")
                        .long("workdays")
                        .takes_value(true)
                        .required(false)
                        .default_value("1,2,3,4,5")
                )
        )
        .get_matches();

    match matches.subcommand_name() {
        Some("today") => {
            println!("Today = {}", date_to_cwver_str(&Local::now().naive_local().date()));
            Ok(())
        }
        Some("convert") => {
            let cw_ver_str = matches
                .subcommand_matches("convert")
                .unwrap()
                .value_of("cw_ver_str")
                .unwrap();
            println!("{} = {:?}", cw_ver_str, cwver_str_to_date(cw_ver_str)?);
            Ok(())
        }
        Some("bisect") => {
            let matches = matches.subcommand_matches("bisect").unwrap();
            let workdays = workdays_to_hashset(matches.value_of("workdays").unwrap())?;
            let (from_str, till_str) = (matches.value_of("from").unwrap(), matches.value_of("till").unwrap());
            let (from, till) = (cwver_str_to_date(from_str)?, cwver_str_to_date(till_str)?);
            let regression_range_in_workdays = count_workdays(&workdays, &from, &till)?;

            println!("Regression Range:");
            println!(
                " {:10}  ➔  {:10} ({} workday(s))\n",
                &from, &till, regression_range_in_workdays
            );

            let middle_of_range = bisect_range(&workdays, &from, &till)?;
            let mut middle_of_range_iter = middle_of_range.iter();

            match middle_of_range.len() {
                0 => {
                    println!("Dates too close to each other, no bisecting necessary");
                }
                1 => {
                    let middle = middle_of_range_iter.next().unwrap();
                    println!("Bisect starting point:");
                    println!(" • {} = {:?}", date_to_cwver_str(&middle), middle);
                }
                2 => {
                    let middle_left = middle_of_range_iter.next().unwrap();
                    let middle_right = middle_of_range_iter.next().unwrap();
                    println!("Two equivaletent bisect starting points:");
                    println!(" • {} = {:?}, or", date_to_cwver_str(&middle_left), middle_left);
                    println!(" • {} = {:?}", date_to_cwver_str(&middle_right), middle_right);
                }
                _ => {
                    panic!("More than 2 dates for bisecting found");
                }
            }
            Ok(())
        }
        None => {
            println!("Today = {}", date_to_cwver_str(&Local::now().naive_local().date()));
            Ok(())
        }
        _ => Err("Unknown subcommand".to_string()),
    }
}

fn bisect_range(workdays: &HashSet<u32>, from: &NaiveDate, till: &NaiveDate) -> Result<OrdSet<NaiveDate>, String> {
    let regression_range_in_workdays: f32 = count_workdays(&workdays, &from, &till)? as f32;

    if regression_range_in_workdays < 2.0 {
        return Ok(ordset!());
    }

    Ok(ordset!(
        jump_n_workdays(&from, (regression_range_in_workdays / 2.0) as u32, &workdays),
        jump_n_workdays(&from, (regression_range_in_workdays / 2.0 + 0.5) as u32, &workdays)
    ))
}

fn jump_n_workdays(from: &NaiveDate, n: u32, workdays: &HashSet<u32>) -> NaiveDate {
    let (mut i, mut date) = (0, from.clone());
    loop {
        if i >= n {
            break date;
        }
        date = next_workday(&workdays, &date);
        i += 1;
    }
}

fn workdays_to_hashset(workdays_of_week: &str) -> Result<HashSet<u32>, String> {
    let mut v = vec![];
    for workday_as_str in workdays_of_week.split(",").collect::<Vec<&str>>() {
        let w = workday_as_str
            .parse::<u32>()
            .map_err(|_| format!("failed to parse workday {}", workday_as_str))?;
        if w < 1 || w > 7 {
            return Err(format!("given workday {} not in range [1-7]", w));
        }
        v.push(w);
    }
    Ok(HashSet::from(v))
}

fn cwver_str_to_date(cw_ver_str: &str) -> Result<NaiveDate, String> {
    let (year, week, day_of_week) =
        parse_cwver_str(cw_ver_str).ok_or_else(|| format!("failed to parse {}", cw_ver_str))?;
    if day_of_week < 1 || day_of_week > 7 {
        return Err(format!("day of week {} out-of-range [1-7]", day_of_week));
    }
    let weekday = chrono::Weekday::from_u32(day_of_week - 1)
        .ok_or_else(|| format!("{} is not a valid day of week", day_of_week))?;
    NaiveDate::from_isoywd_opt(2000 + year, week, weekday)
        .ok_or_else(|| format!("failed to calculate date of {}", cw_ver_str))
}

fn parse_cwver_str(cw_ver_str: &str) -> Option<(i32, u32, u32)> {
    let caps = Regex::new(r"(\d{2})w(\d{2})\.(\d{1})").unwrap().captures(cw_ver_str)?;

    Some((
        caps.get(1)?.as_str().parse().ok()?,
        caps.get(2)?.as_str().parse().ok()?,
        caps.get(3)?.as_str().parse().ok()?,
    ))
}

fn date_to_cwver_str(date: &NaiveDate) -> String {
    let iso_week = date.iso_week();
    format!(
        "{:02}w{:02}.{:01}",
        iso_week.year() % 100,
        iso_week.week(),
        date.weekday().number_from_monday()
    )
}

fn count_workdays(workdays_of_week: &HashSet<u32>, from: &NaiveDate, till: &NaiveDate) -> Result<u32, String> {
    if from > till {
        return Err(format!("{} must be before {} in time", from, till));
    }
    if from == till {
        return Ok(0);
    }

    let mut current = from.clone();
    let mut count = 1;
    loop {
        current = current.succ();
        if &current == till {
            return Ok(count);
        }
        if workdays_of_week.contains(&current.weekday().number_from_monday()) {
            count += 1;
        }
    }
}

fn next_workday(workdays: &HashSet<u32>, from: &NaiveDate) -> NaiveDate {
    let mut next = from.succ();
    loop {
        if workdays.contains(&next.weekday().number_from_monday()) {
            return next;
        }
        next = next.succ();
    }
}

mod tests {
    #[cfg(test)]
    use super::*;

    #[test]
    fn test_parse_cwver() {
        assert_eq!(parse_cwver_str("21w01.2"), Some((21, 1, 2)));
        assert_eq!(parse_cwver_str("00w00.0"), Some((0, 0, 0)));
        assert_eq!(parse_cwver_str("99w99.9"), Some((99, 99, 9)));
        assert_eq!(parse_cwver_str("21w1.0"), None);
    }

    #[test]
    fn test_cwver_str_to_date() {
        assert_eq!(cwver_str_to_date("21w01.1"), Ok(NaiveDate::from_ymd(2021, 01, 04)));
        assert_eq!(cwver_str_to_date("21w10.7"), Ok(NaiveDate::from_ymd(2021, 03, 14)));
        assert_eq!(cwver_str_to_date("21w52.7"), Ok(NaiveDate::from_ymd(2022, 01, 02)));
        assert_eq!(
            cwver_str_to_date("21w52.0"),
            Err("day of week 0 out-of-range [1-7]".to_string())
        );
        assert_eq!(
            cwver_str_to_date("21w00.1"),
            Err("failed to calculate date of 21w00.1".to_string())
        );
        assert_eq!(
            cwver_str_to_date("21w53.1"),
            Err("failed to calculate date of 21w53.1".to_string())
        );
    }

    #[test]
    fn test_date_to_cwver_str() {
        assert_eq!(
            date_to_cwver_str(&NaiveDate::from_ymd(2021, 01, 04)),
            "21w01.1".to_string()
        );
        assert_eq!(
            date_to_cwver_str(&NaiveDate::from_ymd(2021, 03, 14)),
            "21w10.7".to_string()
        );
        assert_eq!(
            date_to_cwver_str(&NaiveDate::from_ymd(2022, 01, 02)),
            "21w52.7".to_string()
        );
    }

    #[test]
    fn test_count_workdays() {
        let commercial_workdays = &hashset![1, 2, 3, 4, 5];
        let max_workdays = &hashset![1, 2, 3, 4, 5, 6, 7];
        assert_eq!(
            count_workdays(
                &commercial_workdays,
                &NaiveDate::from_ymd(2021, 01, 04),
                &NaiveDate::from_ymd(2021, 01, 03)
            ),
            Err("2021-01-04 must be before 2021-01-03 in time".to_string())
        );
        assert_eq!(
            count_workdays(
                &commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 14),
                &NaiveDate::from_ymd(2021, 03, 14)
            ),
            Ok(0)
        );
        assert_eq!(
            count_workdays(
                &commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 12),
                &NaiveDate::from_ymd(2021, 03, 15)
            ),
            Ok(1)
        );
        assert_eq!(
            count_workdays(
                &max_workdays,
                &NaiveDate::from_ymd(2021, 03, 12),
                &NaiveDate::from_ymd(2021, 03, 15)
            ),
            Ok(3)
        );
        assert_eq!(
            count_workdays(
                &max_workdays,
                &NaiveDate::from_ymd(2021, 03, 11),
                &NaiveDate::from_ymd(2021, 03, 12)
            ),
            Ok(1)
        );
    }

    #[test]
    fn test_bisect_range() {
        let commercial_workdays = &hashset![1, 2, 3, 4, 5];
        let max_workdays = &hashset![1, 2, 3, 4, 5, 6, 7];

        assert_eq!(
            bisect_range(
                commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 8),
                &NaiveDate::from_ymd(2021, 03, 9)
            ),
            Ok(ordset!())
        );

        assert_eq!(
            bisect_range(
                commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 8),
                &NaiveDate::from_ymd(2021, 03, 10)
            ),
            Ok(ordset!(NaiveDate::from_ymd(2021, 03, 9)))
        );

        assert_eq!(
            bisect_range(
                commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 8),
                &NaiveDate::from_ymd(2021, 03, 12)
            ),
            Ok(ordset!(NaiveDate::from_ymd(2021, 03, 10)))
        );

        assert_eq!(
            bisect_range(
                commercial_workdays,
                &NaiveDate::from_ymd(2021, 03, 8),
                &NaiveDate::from_ymd(2021, 03, 15)
            ),
            Ok(ordset!(
                NaiveDate::from_ymd(2021, 03, 10),
                NaiveDate::from_ymd(2021, 03, 11)
            ))
        );

        assert_eq!(
            bisect_range(
                max_workdays,
                &NaiveDate::from_ymd(2021, 03, 1),
                &NaiveDate::from_ymd(2021, 03, 7)
            ),
            Ok(ordset!(NaiveDate::from_ymd(2021, 03, 4)))
        );

        assert_eq!(
            bisect_range(
                max_workdays,
                &NaiveDate::from_ymd(2021, 03, 1),
                &NaiveDate::from_ymd(2021, 03, 8)
            ),
            Ok(ordset!(
                NaiveDate::from_ymd(2021, 03, 4),
                NaiveDate::from_ymd(2021, 03, 5)
            ))
        );
    }
}
