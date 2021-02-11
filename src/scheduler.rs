use date_time_parser::DateParser;
use date_time_parser::TimeParser;
use chrono::{NaiveDateTime, Datelike};
use regex::Regex;


pub fn find_time(time_string: &str) -> Option<NaiveDateTime> {
    let time_string = time_string.to_lowercase();
    return match get_exact_datetime(&time_string) {
        Some(datetime) => Some(datetime),
        None => {
            match get_offset_time(&time_string) {
                Some(datetime) => Some(datetime),
                None => {
                    get_fuzzy_time(&time_string)
                }
            }
        }
    }
}

//returns a time in the future based on a 'natural' string
fn get_fuzzy_time(string_time: &str) -> Option<NaiveDateTime> {
    let date = DateParser::parse(string_time);
    let time = TimeParser::parse(string_time);
    match (date, time) {
        (None, None) => return None,
        _ => ()
    }
    //at least one of date/time was successfully parsed
    let time = time.unwrap_or(chrono::NaiveTime::from_hms(0, 0, 0));
    let date = date.unwrap_or(chrono::Local::now().naive_local().date());

    //DILEMMA
    //at this point, we could have something like '7:30'
    //if time did not have an am/pm specifier, then it could potentially be in the past, but that might not be what they meant
    //we can maybe resolve this by checking if the currently parsed date is in the past, 
    //  and if the time component is before noon, try adding 12 hours and see if that puts us in the future
    let mut datetime = chrono::NaiveDateTime::new(date, time);
    let time_now = chrono::Local::now().naive_local();
    if datetime < time_now && time < chrono::NaiveTime::from_hms(12, 0, 0) {
        datetime = datetime + chrono::Duration::hours(12);
    }

    //make sure the date is in the future
    if datetime > time_now {Some(datetime)}
    else {None}
}

//in X <timescale>
//X <timescale> from now
//X <timescale>
//next <timescale>
fn get_offset_time(time_string: &str) -> Option<NaiveDateTime> {

    let mut potential_datetimes: Vec<NaiveDateTime> = Vec::new();
    let time_now = chrono::Local::now().naive_local();

    let re = Regex::new(r"(\d+)\s?(\S+)").unwrap();
    for cap in re.captures_iter(time_string) {
        let offset = match cap[1].parse::<i32>() {
            Ok(offset) => offset,
            _ => continue,
        };
        let time_scale = match str_to_timescale(&cap[2]) {
            Some(time_scale) => time_scale*offset,
            _ => continue,
        };
        potential_datetimes.push(time_now + time_scale);
    }

    let re = Regex::new(r"next\s(\S+)").unwrap();
    for cap in re.captures_iter(time_string) {
        let time_scale = match str_to_timescale(&cap[1]) {
            Some(time_scale) => time_scale,
            _ => continue,
        };
        potential_datetimes.push(time_now + time_scale);
    }
    potential_datetimes = potential_datetimes.into_iter().filter(|element| element > &time_now).collect();
    if potential_datetimes.len() > 0 {
        Some(potential_datetimes[0])
    }
    else {
        None
    }
}

//for getting exact timestamps
//date
//YYYY_MM_DD
//YY_MM_DD
//MM_DD_YYYY
//MM_DD_YY
//MM_DD
//time
//HH:MM:SS
//H:MM:SS
fn get_exact_datetime(time_string: &str) -> Option<NaiveDateTime> {
    let mut potential_dates = Vec::new();
    let mut potential_times = Vec::new();
    //date regexes
    //allowed separators
    let sep_set = r"[/\- \._\\]?";
    //unambiguous yyyy mm dd
    let date_regex_1 = Regex::new(&format!("{}{}{}{}{}", r"(\d{4})", sep_set, r"(\d{2})", sep_set, r"(\d{2})")).unwrap();
    for cap in date_regex_1.captures_iter(time_string) {
        let curr_string = format!("{}-{}-{}", &cap[1], &cap[2], &cap[3]);
        match chrono::NaiveDate::parse_from_str(&curr_string, "%Y-%m-%d") {
            Ok(date) => potential_dates.push(date),
            _ => continue,
        };
    }
    //unambiguous  mm dd yyyy
    let date_regex_2 = Regex::new(&format!("{}{}{}{}{}", r"(\d{2})", sep_set, r"(\d{2})", sep_set, r"(\d{4})")).unwrap();
    for cap in date_regex_2.captures_iter(time_string) {
        let curr_string = format!("{}-{}-{}", &cap[3], &cap[1], &cap[2]);
        match chrono::NaiveDate::parse_from_str(&curr_string, "%Y-%m-%d") {
            Ok(date) => potential_dates.push(date),
            _ => continue,
        };
    }
    //ambiguous, could be yymmdd or mmddyy
    let date_regex_3 = Regex::new(&format!("{}{}{}{}{}", r"(\d{2})", sep_set, r"(\d{2})", sep_set, r"(\d{2})")).unwrap();
    for cap in date_regex_3.captures_iter(time_string) {
        //attempt to determine which one is the year (which ever one is greater than 12)
        let potential_year_1 = match cap[1].parse::<i32>() {
            Ok(year) => year,
            _ => continue,
        };
        if potential_year_1 > 12 || potential_year_1 == 0 {
            //if we are here, then the format is yymmdd
            //TODO: change that hardcoded 20 to get the current century prefix
            let curr_string = format!("20{}-{}-{}", &cap[1], &cap[2], &cap[3]);
            match chrono::NaiveDate::parse_from_str(&curr_string, "%Y-%m-%d") {
                Ok(date) => potential_dates.push(date),
                _ => continue,
            };
        }
        else {
            let potential_year_2 = match cap[2].parse::<i32>() {
                Ok(year) => year,
                _ => continue,
            };
            if potential_year_2 > 12 || potential_year_2 == 0 {
                //if we are here, then the format is mmddyy
                //TODO: change that hardcoded 20 to get the current century prefix
                let curr_string = format!("20{}-{}-{}", &cap[3], &cap[1], &cap[2]);
                match chrono::NaiveDate::parse_from_str(&curr_string, "%Y-%m-%d") {
                    Ok(date) => potential_dates.push(date),
                    _ => continue,
                };
            }
        }
        //if neither of the previous conditions fired, then the date was hopelessly ambiguous
    }
    //year not included
    let date_regex_4 = Regex::new(&format!("{}{}{}", r"(\d{2})", sep_set, r"(\d{2})")).unwrap();
    for cap in date_regex_4.captures_iter(time_string) {
        //check if that date with the current year is in the past, and if it is then add one year
        let current_year = chrono::Local::now().year();
        let curr_string = format!("{}-{}-{}", current_year, &cap[1], &cap[2]);
        match chrono::NaiveDate::parse_from_str(&curr_string, "%Y-%m-%d") {
            Ok(date) if date >= chrono::Local::today().naive_local() => potential_dates.push(date),
            Ok(date) => potential_dates.push(date + chrono::Duration::days(365)),
            _ => continue,
        };
    }

    //dates are extracted, attempt to extract times
    let am_pm_regex = r"(a\.?m?\.?|p\.?m?\.?)";
    let time_regex = Regex::new(&format!(r"{}:{}\s*{}", r"(\d{1}|\d{2})", r"(\d{2})", am_pm_regex)).unwrap();
    for cap in time_regex.captures_iter(time_string) {
        match &cap[1].parse::<i32>() {
            Ok(hour) if *hour < 12 => {
                //check if they specified am or pm
                let hour = match &cap[3].chars().next() {
                    Some('p') => *hour + 12,
                    _ => *hour,
                };
                match chrono::NaiveTime::parse_from_str(&format!("{}:{}", hour, &cap[2]), "%H:%M") {
                    Ok(time) => {
                        potential_times.push(time);
                    },
                    Err(_) =>  {
                        continue;
                    }
                }
            },
            _ => continue,
        }
    }
    //try to extract a time that doesn't have the am/pm specifier
    let time_regex = Regex::new(&format!(r"{}:{}", r"(\d{1}|\d{2})", r"(\d{2})")).unwrap();
    for cap in time_regex.captures_iter(time_string) {
        match chrono::NaiveTime::parse_from_str(&format!("{}:{}", &cap[1], &cap[2]), "%H:%M") {
            Ok(time) => potential_times.push(time),
            _ => continue,
        }
    }

    let now = chrono::Local::now().naive_local();
    if potential_dates.len() < 1 {
        return None
    }
    if potential_times.len() < 1 {
        potential_times.push(now.time());
    }

    //okay, now theoretically potential_dates and potential_times are filled up, and we simply need to find the first pair that is in the future
    for date in &potential_dates {
        for time in &potential_times {
            let datetime = chrono::NaiveDateTime::new(date.clone(), time.clone());
            if datetime > now {
                return Some(datetime)
            }
        }
    }
    None
}

fn str_to_timescale(string: &str) -> Option<chrono::Duration> {
    match string {
        "millisecond" | "milliseconds" => Some(chrono::Duration::milliseconds(1)),
        "second" | "seconds" | "sec" | "secs" => Some(chrono::Duration::seconds(1)),
        "minute" | "minutes" | "min" | "mins" | "minaltatitatude" => Some(chrono::Duration::minutes(1)),
        "hour" | "hours" => Some(chrono::Duration::hours(1)),
        "day" | "days" => Some(chrono::Duration::days(1)),
        "week" | "weeks" => Some(chrono::Duration::weeks(1)),
        "month" | "months" => Some(chrono::Duration::weeks(1)*4),
        "year" | "years" => Some(chrono::Duration::days(1)*365),
        "decade" | "decades" => Some(chrono::Duration::days(1)*365*10),
        "century" | "centuries" => Some(chrono::Duration::days(1)*365*100),
        _ => None,
    }
}