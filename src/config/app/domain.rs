use std::error::Error;
use std::fmt;

static RESERVED_PACKAGE_NAMES: [&'static str; 2] = ["kotlin", "java"];

#[derive(Debug)]
pub enum DomainError {
    Empty,
    NotAsciiAlphanumeric { bad_chars: Vec<char> },
    StartsWithDigit { label: String },
    ReservedPackageName { package_name: String },
    StartsOrEndsWithADot,
    EmptyLabel,
}

impl Error for DomainError {}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "Domain can't be empty"),
            Self::NotAsciiAlphanumeric { bad_chars } => write!(
                f,
                "\"{}\" are not valid ASCII alphanumeric characters",
                bad_chars.into_iter().collect::<String>()
            ),
            Self::ReservedPackageName { package_name } => {
                write!(f, "\"{}\" is reserved and cannot be used", package_name)
            }
            Self::StartsWithDigit { label } => write!(
                f,
                "\"{}\" label starts with a digit, which is invalid",
                label
            ),
            Self::StartsOrEndsWithADot => write!(f, "Domain can't start or end with a dot"),
            Self::EmptyLabel => write!(f, "Labels cannot be empty"),
        }
    }
}

pub fn check_domain_syntax(domain_name: &str) -> Result<(), DomainError> {
    if domain_name.is_empty() {
        return Err(DomainError::Empty);
    }
    if domain_name.starts_with(".") || domain_name.ends_with(".") {
        return Err(DomainError::StartsOrEndsWithADot);
    }
    let labels = domain_name.split(".");
    for label in labels {
        if label.is_empty() {
            return Err(DomainError::EmptyLabel);
        }
        if label.chars().nth(0).unwrap().is_digit(10) {
            return Err(DomainError::StartsWithDigit {
                label: label.to_owned(),
            });
        }
        let mut bad_chars = Vec::new();
        for c in label.chars() {
            if !c.is_ascii_alphanumeric() {
                if !bad_chars.contains(&c) {
                    bad_chars.push(c);
                }
            }
        }
        if !bad_chars.is_empty() {
            return Err(DomainError::NotAsciiAlphanumeric { bad_chars });
        }
    }
    let last_label = domain_name.split(".").last().unwrap();
    if RESERVED_PACKAGE_NAMES.contains(&last_label) {
        return Err(DomainError::ReservedPackageName {
            package_name: last_label.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::rstest;

    #[rstest(
        input,
        case("com.example"),
        case("t2900.e1.s709.t1000"),
        case("kotlin.com"),
        case("java.test")
    )]
    fn test_check_domain_syntax_correct(input: &str) {
        assert_eq!(check_domain_syntax(input).unwrap(), ())
    }

    #[rstest(input, error,
        case("ラスト.テスト", DomainError::NotAsciiAlphanumeric { bad_chars: vec!['ラ', 'ス', 'ト'] }),
        case("test.digits.87", DomainError::StartsWithDigit { label: String::from("87") }),
        case("", DomainError::Empty {}),
        case(".bad.dot.syntax", DomainError::StartsOrEndsWithADot {}),
        case("com.kotlin", DomainError::ReservedPackageName { package_name: String::from("kotlin") }),
        case("com..empty.label", DomainError::EmptyLabel)
    )]
    fn test_check_domain_syntax_error(input: &str, error: DomainError) {
        assert_eq!(
            check_domain_syntax(input).unwrap_err().to_string(),
            error.to_string()
        )
    }
}
