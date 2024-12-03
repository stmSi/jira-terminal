use crate::config;
use crate::jira::api;
use chrono::{DateTime, Datelike, Days, NaiveDate, Weekday};
use colored::*;
use dialoguer::Input;
use std::collections::HashMap;
use std::error::Error;

use std::io::Write;
use std::process::{Command, Stdio};

pub fn log_work(
    ticket: &str,
    time_spent: &str,
    comment: Option<&str>,
    start_time: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let url = format!("issue/{}/worklog", ticket);
    let mut payload = json::object! {
        "timeSpent": time_spent,
        "comment": comment.unwrap_or_default(),
    };

    // Assuming `start_time` is a `Some<&str>` with the input "2024-03-19T14:00:00.000+0000"
    if let Some(start_time_value) = start_time {
        // Attempt to parse the input start time.
        match DateTime::parse_from_str(start_time_value, "%Y-%m-%dT%H:%M:%S%.f%z") {
            Ok(parsed_date) => {
                // Reformat to the exact string format expected by JIRA.
                let formatted_date = parsed_date.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string();
                payload["started"] = formatted_date.into();
            }
            Err(e) => {
                eprintln!("Failed to parse the start time: {}", e);
                return Err(Box::new(e));
            }
        }
    }

    let response = api::post_call(url, payload, 2);

    // Check response status and handle it
    if response.is_ok() {
        println!(
            "{} {}",
            "Successfully logged work on ticket".green(),
            ticket.bold().green()
        );
    } else {
        eprintln!(
            "{} {} {}. {} {}",
            "Failed to log work on ticket".red(),
            ticket.bold().red(),
            "Error:".bold().red(),
            response.unwrap(),
            "\n".bold().red()
        );
    }
    println!();

    Ok(())
}

/// Function to interactively log work with enhanced features
pub fn log_work_interactively() -> Result<(), Box<dyn Error>> {
    let mut tickets = get_own_tickets();
    let mut next_date_str: String = String::new();
    loop {
        let mut start_date_str: String;
        let mut start_date: NaiveDate;
        loop {
            if next_date_str.is_empty() {
                start_date_str = Input::new()
                    .with_prompt("Start date (YYYY-MM-DD)")
                    .interact_text()?;
            } else {
                let next_date = chrono::NaiveDate::parse_from_str(&next_date_str, "%Y-%m-%d")?;
                let next_date_weekday = next_date.weekday();

                start_date_str = Input::new()
                    .with_prompt("Start date (YYYY-MM-DD)")
                    .default(format!(
                        "{}:{}",
                        next_date_weekday.to_string(),
                        next_date_str
                    ))
                    .with_initial_text(next_date_str.clone())
                    .interact_text()?;
            }

            let start_date_result = chrono::NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d");
            if start_date_result.is_ok() {
                start_date = start_date_result.unwrap();
                let weekday = start_date.weekday();
                match weekday {
                    Weekday::Sat | Weekday::Sun => {
                        let confirmation: String = Input::new()
                            .with_prompt(
                                format!("This is Weekend {}... continue? (y/n):", weekday).as_str(),
                            )
                            .interact_text()?;

                        if confirmation.to_lowercase().eq("y") {
                            break;
                        }
                    }
                    _ => break,
                }
            } else {
                eprintln!("Error occurred when converting to Date. ");
            }
        }

        // Determine next day
        let mut next_day = start_date.clone();
        loop {
            next_day = next_day.checked_add_days(Days::new(1)).unwrap();
            let weekday = next_day.weekday();
            if weekday != Weekday::Sat && weekday != Weekday::Sun {
                break;
            }
        }
        next_date_str = next_day.format("%Y-%m-%d").to_string();

        let start_time: String = Input::new()
            .with_prompt("Start time for work log (HH:MM) in New York Eastern timezone")
            .default("09:00".into())
            .with_initial_text("09:00")
            .interact_text()?;

        // format is this 2024-02-01T16:00:21.000-0500
        // Format the input date and time, appending the "-0500" (Eastern timezone)
        let datetime_with_timezone = format!("{}T{}:00.000-0500", start_date_str, start_time);

        // Use `fzf` to select a ticket, assuming a get_tickets function that returns a Vec<String> of ticket options
        let ticket_selection = Command::new("fzf")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        {
            let mut stdin = ticket_selection.stdin.as_ref().unwrap();
            for ticket in tickets.clone().into_iter() {
                writeln!(stdin, "{} - {}", ticket.0, ticket.1)?;
            }
        }

        let output = String::from_utf8(ticket_selection.wait_with_output()?.stdout)?;
        let selected_ticket_id: String;
        let selected_ticket_title: String;
        if output.is_empty() {
            println!("No ticket selected. enter ticket manually");
            loop {
                let ticket: String = Input::new().with_prompt("Enter ticket").interact_text()?;

                // check if ticket exists
                let result = api::get_call_v2(format!("issue/{}", ticket));
                if result.is_err() {
                    eprintln!("Error occurred when searching tickets. ");
                    continue;
                } else {
                    selected_ticket_id = ticket;
                    selected_ticket_title = result.unwrap()["fields"]["summary"].to_string();
                    tickets.insert(selected_ticket_id.clone(), selected_ticket_title.clone());
                    config::add_cached_ticket(
                        selected_ticket_id.clone(),
                        selected_ticket_title.clone(),
                    );
                    break;
                }
            }
        } else {
            selected_ticket_id = output.split_whitespace().next().unwrap().to_string();
            selected_ticket_title = tickets
                .get(&selected_ticket_id)
                .unwrap()
                .as_str()
                .to_string();
        }

        println!(
            "Selected ticket: {} - {}",
            selected_ticket_id, selected_ticket_title
        );

        // ask for timespent
        let timespent: String = Input::new()
            .with_prompt("Time spent (e.g. 1h 30m)")
            .interact_text()?;

        // ask for comment
        let comment: String = Input::new().with_prompt("Comment").interact_text()?;

        println!();
        println!("{}", "-------------------".bold().blue());
        println!(
            "{} {}",
            "Selected ticket:".bold().blue(),
            format!("{} - {}", selected_ticket_id, selected_ticket_title)
                .bold()
                .green()
        );
        println!("{} {}", "Time:".bold().yellow(), datetime_with_timezone);
        println!("{} {}", "Time spent:".bold().yellow(), timespent);
        println!("{} {}", "Comment:".bold().yellow(), comment);
        println!("{}", "-------------------".bold().blue());

        let worklog_result = log_work(
            &selected_ticket_id,
            &timespent,
            Some(&comment),
            Some(&datetime_with_timezone),
        );
        if worklog_result.is_err() {
            eprintln!("Failed to log work. Error: {}", worklog_result.unwrap_err());
        }

        println!("-------------------");
    }

    // Ok(())
}

/// Mock function to represent fetching tickets
/// Implement according to your application's logic
pub fn get_own_tickets() -> HashMap<String, String> {
    println!("Fetching assigned tickets...");
    let json_result = api::get_call_v3("search?jql=assignee=currentUser()".to_string()).unwrap();

    let mut fetched_tickets = HashMap::new();
    for issue in json_result["issues"].members() {
        // title with key
        fetched_tickets.insert(
            issue["key"].to_string(),
            issue["fields"]["summary"].to_string(),
        );
    }

    let mut cached_tickets = config::get_cached_tickets();
    // combine two maps
    cached_tickets.extend(fetched_tickets.clone());

    cached_tickets
}
