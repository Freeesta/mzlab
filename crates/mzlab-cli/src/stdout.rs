//! The standard output of a run: the bytes of `Outcome::stdout` written once at the end, and the exit code when they cannot be written.

use crate::Outcome;

/// Writes the standard output of `o` and gives the exit code: 2 when it cannot be written (a full disk under a redirection must not pass for a
/// complete table), the code of `o` otherwise; a reader that stopped early (`| head`) is not an error.
pub fn write_stdout(o: &Outcome, w: &mut impl std::io::Write) -> u8 {
    match w.write_all(&o.stdout).and_then(|_| w.flush()) {
        Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => {
            eprintln!("error: the standard output could not be written ({e})");
            2
        }
        _ => o.code,
    }
}

#[cfg(test)]
mod tests {

    /// A standard output that fails like a full disk, or like a reader that went away.
    struct Broken(std::io::ErrorKind);

    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(self.0))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_standard_output_that_cannot_be_written_is_an_error_but_a_closed_pipe_is_not() {
        let o = crate::Outcome {
            code: 0,
            stdout: b"id,mz\n".to_vec(),
            stderr: String::new(),
        };
        assert_eq!(
            super::super::write_stdout(&o, &mut Broken(std::io::ErrorKind::StorageFull)),
            2
        );
        assert_eq!(
            super::super::write_stdout(&o, &mut Broken(std::io::ErrorKind::BrokenPipe)),
            0
        );
        let mut v = Vec::new();
        assert_eq!(
            super::super::write_stdout(&crate::Outcome { code: 1, ..o }, &mut v),
            1
        );
        assert_eq!(v, b"id,mz\n");
    }
}
