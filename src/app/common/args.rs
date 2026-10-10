use std::str::FromStr;

pub struct CliArgs(Vec<String>);

impl CliArgs {
    pub fn from_env() -> Self {
        Self(std::env::args().collect())
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    pub fn has(&self, flag: &str) -> bool {
        self.iter().any(|arg| arg == flag)
    }

    pub fn value(&self, flag: &str) -> Option<&str> {
        self.0
            .windows(2)
            .filter(|pair| pair[0] == flag)
            .map(|pair| pair[1].as_str())
            .last()
    }

    pub fn parsed<T: FromStr>(&self, flag: &str) -> Option<T> {
        self.0
            .windows(2)
            .filter(|pair| pair[0] == flag)
            .filter_map(|pair| pair[1].parse().ok())
            .last()
    }
}

impl<S: Into<String>> FromIterator<S> for CliArgs {
    fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Self {
        Self(iter.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_valid_value_wins() {
        let args: CliArgs = ["bin", "--port", "1", "--port", "nope", "--port", "3", "--flag"]
            .into_iter()
            .collect();
        assert_eq!(args.parsed::<u16>("--port"), Some(3));
        assert_eq!(args.value("--port"), Some("3"));
        assert!(args.has("--flag"));
        assert_eq!(args.value("--flag"), None);
        assert_eq!(args.parsed::<u16>("--missing"), None);
    }

    #[test]
    fn invalid_trailing_value_keeps_previous_valid_one() {
        let args: CliArgs = ["bin", "--port", "7", "--port", "bad"].into_iter().collect();
        assert_eq!(args.parsed::<u16>("--port"), Some(7));
    }
}
