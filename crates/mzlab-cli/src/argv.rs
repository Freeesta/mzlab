//! One check for the commands that take positional words only: an argument that looks like an option is an error, never a file name.

use mzlab_core::Error;

/// `args` unchanged when none is an unknown option (a bare `-` is stdin and stays); otherwise `err.cli.usage` naming the first.
pub(crate) fn no_options(args: &[String]) -> Result<(), Error> {
    match args.iter().find(|a| a.len() > 1 && a.starts_with('-')) {
        Some(a) => Err(Error::new("err.cli.usage").with("message", format!("unknown option {a}"))),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::no_options;

    #[test]
    fn options_are_rejected_and_stdin_is_not() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(no_options(&v(&["a.json", "-"])).is_ok());
        assert!(no_options(&v(&["a.json", "--bogus"])).is_err());
        assert!(no_options(&v(&["-x"])).is_err());
    }
}
