//! Duration and input parsing for break timers.
//!
//! This module provides flexible parsing of natural language duration input,
//! supporting multiple formats including standard time units (`5m`, `1h30m`),
//! colon-formatted times (`5:30`, `1:30:45`), and mixed formats.

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Parse error: {}", self.0)
    }
}

impl Error for ParseError {}

#[derive(Debug)]
enum Token {
    Number(u64),
    Unit(String),
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let input = input.trim().to_lowercase();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_number = false;

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            if !in_number && !current.is_empty() {
                // Transitioning from text to number, save the text token
                tokens.push(Token::Unit(current.clone()));
                current.clear();
            }
            in_number = true;
            current.push(ch);
        } else if ch.is_ascii_alphabetic() {
            if in_number && !current.is_empty() {
                // Transitioning from number to text, save the number token
                let num: u64 = current
                    .parse()
                    .map_err(|_| ParseError(format!("Invalid number: {}", current)))?;
                tokens.push(Token::Number(num));
                current.clear();
            }
            in_number = false;
            current.push(ch);
        } else if ch.is_whitespace() {
            // Save current token if any
            if !current.is_empty() {
                if in_number {
                    let num: u64 = current
                        .parse()
                        .map_err(|_| ParseError(format!("Invalid number: {}", current)))?;
                    tokens.push(Token::Number(num));
                } else {
                    tokens.push(Token::Unit(current.clone()));
                }
                current.clear();
                in_number = false;
            }
        } else {
            // Allow other characters as part of message text (emoji, punctuation, etc.)
            // If we're in a number, save it first
            if in_number && !current.is_empty() {
                let num: u64 = current
                    .parse()
                    .map_err(|_| ParseError(format!("Invalid number: {}", current)))?;
                tokens.push(Token::Number(num));
                current.clear();
                in_number = false;
            }
            // Add character to current token (will be treated as Unit/message text)
            current.push(ch);
        }
    }

    // Save final token
    if !current.is_empty() {
        if in_number {
            let num: u64 = current
                .parse()
                .map_err(|_| ParseError(format!("Invalid number: {}", current)))?;
            tokens.push(Token::Number(num));
        } else {
            tokens.push(Token::Unit(current));
        }
    }

    Ok(tokens)
}

fn parse_unit(unit: &str) -> Result<u64, ParseError> {
    match unit {
        // Hours
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(3600),
        // Minutes
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(60),
        // Seconds
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(1),
        _ => Err(ParseError(format!("Unknown time unit: '{}'", unit))),
    }
}

/// Parse colon-formatted time (h:m:s, m:s, or just s)
/// Examples: "1:30:45" -> 5445, "5:30" -> 330, "45" -> 45
fn parse_colon_time(s: &str) -> Result<u64, ParseError> {
    let parts: Vec<&str> = s.split(':').collect();

    match parts.len() {
        1 => {
            // Just seconds (though this shouldn't have a colon)
            let secs: u64 = parts[0].parse()
                .map_err(|_| ParseError(format!("Invalid seconds: {}", parts[0])))?;
            Ok(secs)
        }
        2 => {
            // minutes:seconds
            let mins: u64 = parts[0].parse()
                .map_err(|_| ParseError(format!("Invalid minutes: {}", parts[0])))?;
            let secs: u64 = parts[1].parse()
                .map_err(|_| ParseError(format!("Invalid seconds: {}", parts[1])))?;
            Ok(mins * 60 + secs)
        }
        3 => {
            // hours:minutes:seconds
            let hours: u64 = parts[0].parse()
                .map_err(|_| ParseError(format!("Invalid hours: {}", parts[0])))?;
            let mins: u64 = parts[1].parse()
                .map_err(|_| ParseError(format!("Invalid minutes: {}", parts[1])))?;
            let secs: u64 = parts[2].parse()
                .map_err(|_| ParseError(format!("Invalid seconds: {}", parts[2])))?;
            Ok(hours * 3600 + mins * 60 + secs)
        }
        _ => Err(ParseError(format!("Invalid time format: {}", s)))
    }
}

/// Check if a string looks like a colon time format
fn is_colon_time(s: &str) -> bool {
    if !s.contains(':') {
        return false;
    }

    // Must be all digits and colons
    s.chars().all(|c| c.is_ascii_digit() || c == ':')
}

/// Parses user input that mixes duration components with message text.
///
/// This function accepts flexible, natural language input for specifying break timers.
/// It extracts all duration components (standard units and colon-formatted times) and
/// treats remaining text as the timer message.
///
/// # Supported Duration Formats
///
/// - **Standard units**: `5m`, `1h`, `30s`, `5minutes`, `1hour`, `30seconds`
/// - **Colon format**: `5:30` (5 min 30 sec), `1:30:45` (1 hr 30 min 45 sec)
/// - **Mixed formats**: `1h 30m 2:15 message` combines all duration types
///
/// # Examples
///
/// ```
/// # use break::parser::parse_input;
/// // Simple format
/// let (duration, msg) = parse_input("5m get coffee").unwrap();
/// assert_eq!(duration, 300); // 5 minutes in seconds
/// assert_eq!(msg, "get coffee");
///
/// // Colon format
/// let (duration, msg) = parse_input("1:30:45 long break").unwrap();
/// assert_eq!(duration, 5445); // 1h 30m 45s in seconds
///
/// // Mixed formats
/// let (duration, msg) = parse_input("15mins 1 hour 20s take a break").unwrap();
/// assert_eq!(duration, 4520); // Sum of all durations
/// assert_eq!(msg, "take a break");
/// ```
///
/// # Returns
///
/// - `Ok((u64, String))` - Duration in seconds and the message text
/// - `Err(ParseError)` - If no valid duration found, no message found, or invalid format
///
/// # Errors
///
/// Returns `ParseError` if:
/// - No duration components found in input
/// - No message text found (duration only)
/// - Invalid time unit or format
/// - Empty input
pub fn parse_input(input: &str) -> Result<(u64, String), ParseError> {
    // First, scan for colon-formatted times
    let words: Vec<&str> = input.split_whitespace().collect();
    let mut colon_duration = 0u64;
    let mut remaining_input = Vec::new();

    for word in words {
        if is_colon_time(word) {
            colon_duration += parse_colon_time(word)?;
        } else {
            remaining_input.push(word);
        }
    }

    // If we only had colon time and no other input, that's an error (no message)
    if remaining_input.is_empty() && colon_duration > 0 {
        return Err(ParseError("No message found in input".to_string()));
    }

    // Parse the remaining input for standard duration formats
    let remaining_str = remaining_input.join(" ");
    let tokens = tokenize(&remaining_str)?;

    // Allow empty tokens if we got duration from colon format
    if tokens.is_empty() && colon_duration == 0 {
        return Err(ParseError("Empty input".to_string()));
    }

    let mut total_seconds = colon_duration; // Start with colon duration
    let mut message_parts = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Number(num) => {
                // Look for a unit after the number
                if i + 1 < tokens.len() {
                    if let Token::Unit(unit) = &tokens[i + 1] {
                        // Check if this is a valid time unit
                        if let Ok(multiplier) = parse_unit(unit) {
                            total_seconds += num * multiplier;
                            i += 2;
                            continue;
                        }
                        // Not a time unit, treat as message text
                        message_parts.push(num.to_string());
                        message_parts.push(unit.clone());
                        i += 2;
                        continue;
                    }
                }
                // No unit following, treat number as message text
                message_parts.push(num.to_string());
                i += 1;
            }
            Token::Unit(unit) => {
                // Standalone unit, treat as message text
                message_parts.push(unit.clone());
                i += 1;
            }
        }
    }

    if total_seconds == 0 {
        return Err(ParseError("No valid duration found in input".to_string()));
    }

    let message = message_parts.join(" ");
    if message.is_empty() {
        return Err(ParseError("No message found in input".to_string()));
    }

    Ok((total_seconds, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Basic duration parsing with simple units
    #[test]
    fn test_simple_short_units() {
        let (duration, message) = parse_input("5m break").unwrap();
        assert_eq!(duration, 300);
        assert_eq!(message, "break");

        let (duration, message) = parse_input("timer 1h").unwrap();
        assert_eq!(duration, 3600);
        assert_eq!(message, "timer");

        let (duration, message) = parse_input("30s reminder").unwrap();
        assert_eq!(duration, 30);
        assert_eq!(message, "reminder");
    }

    #[test]
    fn test_simple_long_units() {
        let (duration, _) = parse_input("5minutes break").unwrap();
        assert_eq!(duration, 300);

        let (duration, _) = parse_input("1hour timer").unwrap();
        assert_eq!(duration, 3600);

        let (duration, _) = parse_input("30seconds go").unwrap();
        assert_eq!(duration, 30);

        let (duration, _) = parse_input("2hrs meeting").unwrap();
        assert_eq!(duration, 7200);

        let (duration, _) = parse_input("45mins lunch").unwrap();
        assert_eq!(duration, 2700);
    }

    // Combined durations
    #[test]
    fn test_combined_short_units() {
        let (duration, _) = parse_input("1h30m break").unwrap();
        assert_eq!(duration, 5400);

        let (duration, _) = parse_input("2h15m30s meeting").unwrap();
        assert_eq!(duration, 8130);
    }

    #[test]
    fn test_combined_long_units() {
        let (duration, _) = parse_input("1hour30minutes break").unwrap();
        assert_eq!(duration, 5400);

        let (duration, _) = parse_input("msg 2hours 15minutes 30seconds").unwrap();
        assert_eq!(duration, 8130);

        let (duration, _) = parse_input("1 hour 30 minutes break").unwrap();
        assert_eq!(duration, 5400);
    }

    #[test]
    fn test_mixed_units() {
        let (duration, _) = parse_input("1h 30min break").unwrap();
        assert_eq!(duration, 5400);

        let (duration, _) = parse_input("5 hours 30m timer").unwrap();
        assert_eq!(duration, 19800);

        let (duration, _) = parse_input("1hour30m break").unwrap();
        assert_eq!(duration, 5400);

        let (duration, _) = parse_input("msg 1second 5h 30min").unwrap();
        assert_eq!(duration, 19801);
    }

    // Case insensitivity
    #[test]
    fn test_case_insensitive() {
        let (duration, _) = parse_input("5M break").unwrap();
        assert_eq!(duration, 300);

        let (duration, _) = parse_input("1H timer").unwrap();
        assert_eq!(duration, 3600);

        let (duration, _) = parse_input("30S go").unwrap();
        assert_eq!(duration, 30);

        let (duration, _) = parse_input("5Minutes break").unwrap();
        assert_eq!(duration, 300);

        let (duration, _) = parse_input("1HOUR timer").unwrap();
        assert_eq!(duration, 3600);
    }

    // Duration and message in various positions
    #[test]
    fn test_parse_input_mixed() {
        let (duration, message) = parse_input("15mins 1 hour 20s take a break").unwrap();
        assert_eq!(duration, 15 * 60 + 3600 + 20); // 4520 seconds
        assert_eq!(message, "take a break");
    }

    #[test]
    fn test_parse_input_duration_first() {
        let (duration, message) = parse_input("5m coffee time").unwrap();
        assert_eq!(duration, 300);
        assert_eq!(message, "coffee time");
    }

    #[test]
    fn test_parse_input_duration_last() {
        let (duration, message) = parse_input("get coffee 5m").unwrap();
        assert_eq!(duration, 300);
        assert_eq!(message, "get coffee");
    }

    #[test]
    fn test_parse_input_multiple_durations() {
        let (duration, message) = parse_input("wait 5m and then 10s more for tea").unwrap();
        assert_eq!(duration, 5 * 60 + 10); // 310 seconds
        assert_eq!(message, "wait and then more for tea");
    }

    #[test]
    fn test_parse_input_message_with_numbers() {
        let (duration, message) = parse_input("5m call 123 people").unwrap();
        assert_eq!(duration, 300);
        assert_eq!(message, "call 123 people");
    }

    #[test]
    fn test_parse_input_complex() {
        let (duration, message) = parse_input("1h 30m break for lunch at 12").unwrap();
        assert_eq!(duration, 3600 + 1800); // 5400 seconds
        assert_eq!(message, "break for lunch at 12");
    }

    // Error cases
    #[test]
    fn test_parse_input_errors() {
        // No duration
        assert!(parse_input("just a message").is_err());
        // No message
        assert!(parse_input("5m").is_err());
        assert!(parse_input("1h 30m").is_err());
        // Empty
        assert!(parse_input("").is_err());
        // Invalid unit
        assert!(parse_input("5x message").is_err());
    }

    // Colon time format tests
    #[test]
    fn test_colon_format_minutes_seconds() {
        let (duration, message) = parse_input("5:30 tea is ready").unwrap();
        assert_eq!(duration, 5 * 60 + 30); // 330 seconds
        assert_eq!(message, "tea is ready");
    }

    #[test]
    fn test_colon_format_hours_minutes_seconds() {
        let (duration, message) = parse_input("1:30:45 coffee break").unwrap();
        assert_eq!(duration, 1 * 3600 + 30 * 60 + 45); // 5445 seconds
        assert_eq!(message, "coffee break");
    }

    #[test]
    fn test_colon_format_with_leading_zeros() {
        let (duration, message) = parse_input("05:50:55 timer").unwrap();
        assert_eq!(duration, 5 * 3600 + 50 * 60 + 55); // 21655 seconds
        assert_eq!(message, "timer");
    }

    #[test]
    fn test_colon_format_message_first() {
        let (duration, message) = parse_input("reminder 0:30").unwrap();
        assert_eq!(duration, 30); // 30 seconds
        assert_eq!(message, "reminder");
    }

    #[test]
    fn test_colon_format_mixed_with_standard() {
        // Can combine colon format with standard duration units
        let (duration, message) = parse_input("1:30 5m reminder").unwrap();
        assert_eq!(duration, 90 + 300); // 390 seconds
        assert_eq!(message, "reminder");
    }

    #[test]
    fn test_colon_format_multiple() {
        let (duration, message) = parse_input("1:00 2:30 break").unwrap();
        assert_eq!(duration, 60 + 150); // 210 seconds
        assert_eq!(message, "break");
    }

    #[test]
    fn test_colon_format_errors() {
        // No message
        assert!(parse_input("5:30").is_err());
        // Invalid format
        assert!(parse_input("5:30:45:10 message").is_err());
        // Non-numeric
        assert!(parse_input("5:3a message").is_err());
    }
}
