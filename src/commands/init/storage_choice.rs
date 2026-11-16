//! Whether a run of `rex init` keeps the record of jobs: from a flag, from a config
//! that already keeps it, or from one question to the person at a terminal.

use std::io::{self, BufRead, Write};

/// The question that a run with neither storage flag asks at a terminal.
pub(super) const QUESTION: &str = "rex init: keep a record of jobs in .rex/rex.db? It is needed by rex jobs, rex job show, recall, feedback, the cost of record and rex master resume; without it the record of a job ends with its session, and rex watch shows only the snapshots that sessions wrote in the last week. [Y/n] ";

/// The line a run prints when storage stays off and the person did not choose that with `--without-storage`.
pub(super) const OFF_NOTICE: &str = "rex init: storage is off, so rex jobs, rex job show, recall, feedback, the cost of record and rex master resume have nothing to read, and rex watch shows only the snapshots that sessions wrote in the last week; run rex init --with-storage to turn it on";

/// What a run asks of storage, before any question.
pub(super) struct StorageFlags {
    /// `--with-storage`.
    pub(super) is_with: bool,
    /// `--without-storage`.
    pub(super) is_without: bool,
    /// The config already keeps the record: a later run never turns it off.
    pub(super) was_on: bool,
    /// Both standard input and standard output are a terminal, so a person can answer.
    pub(super) is_terminal: bool,
}

/// Whether storage is on after this run. A flag or a config that already keeps
/// the record decides at once; otherwise a person at a terminal is asked, with
/// yes as the answer to an empty line or the end of input, and a run with no
/// terminal keeps storage off.
///
/// # Errors
/// The error of writing the question or reading the answer.
pub(super) fn choose(
    flags: &StorageFlags,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    if flags.is_with || flags.was_on {
        return Ok(true);
    }
    if flags.is_without || !flags.is_terminal {
        return Ok(false);
    }
    loop {
        output.write_all(QUESTION.as_bytes())?;
        output.flush()?;
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(true);
        }
        match line.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => output.write_all(b"rex init: answer y or n\n")?,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(is_with: bool, is_without: bool, was_on: bool, is_terminal: bool) -> StorageFlags {
        StorageFlags {
            is_with,
            is_without,
            was_on,
            is_terminal,
        }
    }

    fn answer(flags: &StorageFlags, typed: &str) -> (bool, String) {
        let mut output = Vec::new();
        let chosen = choose(flags, &mut typed.as_bytes(), &mut output).expect("choose");
        (chosen, String::from_utf8(output).expect("utf8"))
    }

    #[test]
    fn a_terminal_run_with_no_flag_asks_and_an_empty_answer_means_yes() {
        let (chosen, asked) = answer(&flags(false, false, false, true), "\n");
        assert!(chosen);
        assert_eq!(asked, QUESTION);
    }

    #[test]
    fn a_terminal_run_takes_no_and_asks_again_after_another_answer() {
        let (chosen, asked) = answer(&flags(false, false, false, true), "maybe\nN\n");
        assert!(!chosen);
        assert_eq!(
            asked,
            format!("{QUESTION}rex init: answer y or n\n{QUESTION}")
        );
    }

    #[test]
    fn the_end_of_input_at_the_question_means_yes() {
        assert!(answer(&flags(false, false, false, true), "").0);
    }

    #[test]
    fn a_flag_or_a_config_that_keeps_the_record_asks_nothing() {
        assert_eq!(
            answer(&flags(true, false, false, true), "n\n"),
            (true, String::new())
        );
        assert_eq!(
            answer(&flags(false, true, false, true), "y\n"),
            (false, String::new())
        );
        assert_eq!(
            answer(&flags(false, true, true, true), "n\n"),
            (true, String::new())
        );
    }

    #[test]
    fn a_run_with_no_terminal_keeps_storage_off_and_asks_nothing() {
        assert_eq!(
            answer(&flags(false, false, false, false), "y\n"),
            (false, String::new())
        );
    }
}
