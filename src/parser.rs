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
            return Err(ParseError(format!("Invalid character: '{}'", ch)));
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

pub fn parse_duration(input: &str) -> Result<u64, ParseError> {
    let tokens = tokenize(input)?;

    if tokens.is_empty() {
        return Err(ParseError("Empty duration".to_string()));
    }

    let mut total_seconds = 0u64;
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::Number(num) => {
                // Look for a unit after the number
                if i + 1 < tokens.len() {
                    if let Token::Unit(unit) = &tokens[i + 1] {
                        let multiplier = parse_unit(unit)?;
                        total_seconds += num * multiplier;
                        i += 2;
                        continue;
                    }
                }
                return Err(ParseError(format!(
                    "Number {} must be followed by a unit (e.g., h, m, s, hour, minute, second)",
                    num
                )));
            }
            Token::Unit(unit) => {
                return Err(ParseError(format!(
                    "Unit '{}' must be preceded by a number",
                    unit
                )));
            }
        }
    }

    if total_seconds == 0 {
        return Err(ParseError("Duration must be greater than 0".to_string()));
    }

    Ok(total_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_short() {
        assert_eq!(parse_duration("5m").unwrap(), 300);
        assert_eq!(parse_duration("1h").unwrap(), 3600);
        assert_eq!(parse_duration("30s").unwrap(), 30);
    }

    #[test]
    fn test_parse_simple_long() {
        assert_eq!(parse_duration("5minutes").unwrap(), 300);
        assert_eq!(parse_duration("1hour").unwrap(), 3600);
        assert_eq!(parse_duration("30seconds").unwrap(), 30);
        assert_eq!(parse_duration("2hrs").unwrap(), 7200);
        assert_eq!(parse_duration("45mins").unwrap(), 2700);
    }

    #[test]
    fn test_parse_combined_short() {
        assert_eq!(parse_duration("1h30m").unwrap(), 5400);
        assert_eq!(parse_duration("2h15m30s").unwrap(), 8130);
    }

    #[test]
    fn test_parse_combined_long() {
        assert_eq!(parse_duration("1hour30minutes").unwrap(), 5400);
        assert_eq!(parse_duration("2hours 15minutes 30seconds").unwrap(), 8130);
        assert_eq!(parse_duration("1 hour 30 minutes").unwrap(), 5400);
    }

    #[test]
    fn test_parse_mixed() {
        assert_eq!(parse_duration("1h 30min").unwrap(), 5400);
        assert_eq!(parse_duration("5 hours 30m").unwrap(), 19800);
        assert_eq!(parse_duration("1hour30m").unwrap(), 5400);
        assert_eq!(parse_duration("1second 5h 30min").unwrap(), 19801);
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert_eq!(parse_duration("5M").unwrap(), 300);
        assert_eq!(parse_duration("1H").unwrap(), 3600);
        assert_eq!(parse_duration("30S").unwrap(), 30);
        assert_eq!(parse_duration("5Minutes").unwrap(), 300);
        assert_eq!(parse_duration("1HOUR").unwrap(), 3600);
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("5").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("m5").is_err());
        assert!(parse_duration("5x").is_err());
    }
}
