#[derive(Debug, Clone)]
pub enum CharError {
    ExpectedChar { expected: char, got: char },
    UnexpectedChar { expected: String, got: char },
    CharFailedPredicate { got: char, name: String },
    ExpectedToken(String),
    EndOfInput,
}

pub fn is_char(c: char) -> impl Fn(&str) -> Result<(&str, char), CharError> {
    move |input| {
        if let Some(x) = input.chars().next() {
            if x == c {
                Ok((&input[1..], c))
            } else {
                Err(CharError::ExpectedChar {
                    expected: c,
                    got: x,
                })
            }
        } else {
            Err(CharError::EndOfInput)
        }
    }
}

pub fn not_char<'a, 'b>(s: &'a str) -> impl Fn(&'b str) -> Result<(&'b str, char), CharError> + 'a {
    move |input| {
        if let Some(c) = input.chars().next() {
            if !s.contains(c) {
                Ok((&input[1..], c))
            } else {
                Err(CharError::UnexpectedChar {
                    expected: s.to_string(),
                    got: c,
                })
            }
        } else {
            Err(CharError::EndOfInput)
        }
    }
}

pub fn token<'a, 'b>(s: &'a str) -> impl Fn(&'b str) -> Result<(&'b str, &'a str), CharError> {
    move |input| {
        if input.strip_prefix(s).is_some() {
            Ok((&input[s.len()..], s))
        } else {
            Err(CharError::ExpectedToken(s.to_string()))
        }
    }
}

pub fn alpha(input: &str) -> Result<(&str, char), CharError> {
    char_predicate(char::is_alphabetic, "alphabetic".to_string())(input)
}

pub fn numeric(input: &str) -> Result<(&str, char), CharError> {
    char_predicate(char::is_numeric, "numeric".to_string())(input)
}

pub fn alphanumeric(input: &str) -> Result<(&str, char), CharError> {
    char_predicate(char::is_alphanumeric, "alphanumeric".to_string())(input)
}

pub fn whitespace(input: &str) -> Result<(&str, char), CharError> {
    char_predicate(char::is_whitespace, "whitespace".to_string())(input)
}

pub fn char_predicate_or(
    a: impl Fn(&str) -> Result<(&str, char), CharError>,
    b: impl Fn(&str) -> Result<(&str, char), CharError>,
) -> impl Fn(&str) -> Result<(&str, char), CharError> {
    move |input| {
        if let Ok(x) = a(input) {
            Ok(x)
        } else {
            b(input)
        }
    }
}

pub fn char_predicate(
    f: fn(char) -> bool,
    name: String,
) -> impl Fn(&str) -> Result<(&str, char), CharError> {
    move |input| {
        if let Some(c) = input.chars().next() {
            if f(c) {
                Ok((&input[1..], c))
            } else {
                Err(CharError::CharFailedPredicate {
                    got: c,
                    name: name.to_string(),
                })
            }
        } else {
            Err(CharError::EndOfInput)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::combinator::*;

    #[test]
    fn test_numeric() {
        {
            let (s, n) = numeric("1234").unwrap();
            assert_eq!("234", s);
            assert_eq!(n, '1');
        }
        {
            let (s, n) = map(repeat1(numeric), |x| x.iter().collect::<String>())("1234").unwrap();
            assert_eq!("", s);
            assert_eq!("1234", n);
        }
        {
            let (s, n) =
                map(repeat1(numeric), |x| x.iter().collect::<String>())("1234abcd").unwrap();
            assert_eq!("abcd", s);
            assert_eq!("1234", n);
        }
    }

    #[test]
    fn test_alpha() {
        {
            let (s, n) = alpha("xxxx").unwrap();
            assert_eq!("xxx", s);
            assert_eq!(n, 'x');
        }
        {
            let (s, n) = map(repeat1(alpha), |x| x.iter().collect::<String>())("abcdef").unwrap();
            assert_eq!("", s);
            assert_eq!("abcdef", n);
        }
        {
            let (s, n) = map(repeat1(alpha), |x| x.iter().collect::<String>())("xyz:123").unwrap();
            assert_eq!(":123", s);
            assert_eq!("xyz", n);
        }
    }

    #[test]
    fn test_alphanumeric() {
        {
            let (s, n) = alphanumeric("a7").unwrap();
            assert_eq!("7", s);
            assert_eq!(n, 'a');
        }
        {
            let (s, n) = alphanumeric("9e").unwrap();
            assert_eq!("e", s);
            assert_eq!(n, '9');
        }
        {
            let (s, n) =
                map(repeat1(alphanumeric), |x| x.iter().collect::<String>())("abc123zxy987")
                    .unwrap();
            assert_eq!("", s);
            assert_eq!("abc123zxy987", n);
        }
        {
            let (s, n) =
                map(repeat1(alphanumeric), |x| x.iter().collect::<String>())("a1b2c3::9z").unwrap();
            assert_eq!("::9z", s);
            assert_eq!("a1b2c3", n);
        }
    }

    #[test]
    fn test_whitespace() {
        for c in " \n\r\t".chars() {
            let str_c = c.to_string();
            let (s, n) = whitespace(&str_c).unwrap();
            assert_eq!("", s);
            assert_eq!(c, n);
        }
        {
            let (s, n) =
                map(repeat1(whitespace), |x| x.iter().collect::<String>())("\n\n  \t\t").unwrap();
            assert_eq!("", s);
            assert_eq!("\n\n  \t\t", n);
        }
        {
            let (s, n) =
                map(repeat1(whitespace), |x| x.iter().collect::<String>())("   \nabc\n\n").unwrap();
            assert_eq!("abc\n\n", s);
            assert_eq!("   \n", n);
        }
    }

    #[test]
    fn test_not_char() {
        let (s, n) = map(repeat1(not_char("x")), |x| x.iter().collect::<String>())("foox").unwrap();
        assert_eq!("x", s);
        assert_eq!("foo", n);
    }
}
