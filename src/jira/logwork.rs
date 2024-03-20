use crate::jira::api;
use std::collections::HashMap;
use std::error::Error;
use dialoguer::Input;
use chrono::DateTime;

// use dialoguer::{Input, Select};
use std::process::{Command, Stdio};
use std::io::{self, Write};

pub fn log_work(ticket: &str, time_spent: &str, comment: Option<&str>, start_time: Option<&str>) -> Result<(), Box<dyn Error>> {
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
            },
            Err(e) => {
                eprintln!("Failed to parse the start time: {}", e);
                return Err(Box::new(e));
            }
        }
    }

    let response = api::post_call(url, payload, 2);
    
    // Check response status and handle it
    if response.is_ok() {
        println!("Successfully logged work on ticket {}", ticket);
    } else {
        eprintln!("Failed to log work on ticket {}. Error: {}", ticket, response.unwrap());
    }

    Ok(())
}


/// Function to interactively log work with enhanced features
pub fn log_work_interactively() -> Result<(), Box<dyn Error>> {
    let mut tickets = get_own_tickets();

    let start_date: String = Input::new()
        .with_prompt("Start date (YYYY-MM-DD)")
        .interact_text()?;

    loop {
        let start_time: String = Input::new()
            .with_prompt("Start time for work log (HH:MM) in New York Eastern timezone")
            .interact_text()?;

        // format is this 2024-02-01T16:00:21.000-0500
        // Format the input date and time, appending the "-0500" (Eastern timezone)
        let datetime_with_timezone = format!("{}T{}:00.000-0500", start_date, start_time);

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
        let selected_ticket: String;

        if output.is_empty() {
            println!("No ticket selected. enter ticket manually");
            loop {
                let ticket: String = Input::new()
                    .with_prompt("Enter ticket")
                    .interact_text()?;

                // check if ticket exists
                let result = api::get_call_v2(format!("issue/{}", ticket));
                if result.is_err() {
                    eprintln!("Error occurred when searching tickets. ");
                    continue;
                }
                if result.unwrap().is_null() {
                    eprintln!("Ticket does not exist. ");
                    continue;
                } else {
                    selected_ticket = ticket;
                    let ticket_title = tickets.get(&selected_ticket).unwrap();
                    println!("Selected ticket: {} - {}", selected_ticket, ticket_title);
                    tickets.insert(selected_ticket.clone(), ticket_title.clone());
                    break;
                }
            }
        } else {
            selected_ticket = output.split_whitespace().next().unwrap().to_string();
        }
        
        println!("Selected ticket: {}", selected_ticket);

        println!("Time: {}", datetime_with_timezone);

        // Parse and adjust time for logging

        // Ask if the user wants to continue logging for the same date
        println!("Log another entry for the same date? (y/N)");
        let mut decision = String::new();
        io::stdin().read_line(&mut decision)?;
        if decision.trim().to_lowercase() != "y" {
            break;
        }
    }

    Ok(())
}

/// Mock function to represent fetching tickets
/// Implement according to your application's logic
fn get_own_tickets() -> HashMap<String, String> {
    println!("Fetching assigned tickets...");
    let json_result = api::get_call_v3("search?jql=assignee=currentUser()".to_string())
        .unwrap();

    let mut tickets = HashMap::new();
    for issue in json_result["issues"].members() {
        // title with key
        tickets.insert(
            issue["key"].to_string(),
            issue["fields"]["summary"].to_string(),
        );
    }

    tickets

}
