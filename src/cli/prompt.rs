//! Asking the person at the terminal; scripts and agents are never asked.
use std::io::{self, IsTerminal, Write};

/// The terminal as a decision needs it: is a person there, and what do they answer.
pub(super) trait Console {
    fn is_interactive(&self) -> bool;
    fn confirm(&mut self, question: &str) -> io::Result<bool>;
}

/// The process's real stdin, stdout and stderr.
pub(super) struct TerminalConsole;

impl Console for TerminalConsole {
    fn is_interactive(&self) -> bool {
        io::stdin().is_terminal() && io::stdout().is_terminal() && io::stderr().is_terminal()
    }

    fn confirm(&mut self, question: &str) -> io::Result<bool> {
        let mut stderr = io::stderr().lock();
        write!(stderr, "{question} [y/N] ")?;
        stderr.flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        Ok(answer.trim().eq_ignore_ascii_case("y"))
    }
}

/// Whether to apply freshly staged changes now.
///
/// Only a person at a terminal is asked. Programs choose up front with `--apply` or `--yes`,
/// which write without staging, so JSON output and pipes never apply here.
pub(super) fn wants_apply_now(
    json: bool,
    console: &mut impl Console,
    staged: usize,
) -> io::Result<bool> {
    if json || !console.is_interactive() {
        return Ok(false);
    }
    let noun = if staged == 1 { "change" } else { "changes" };
    console.confirm(&format!("Apply {staged} staged {noun}?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeConsole {
        interactive: bool,
        answer: bool,
        asked: Vec<String>,
    }

    impl FakeConsole {
        fn new(interactive: bool, answer: bool) -> Self {
            Self {
                interactive,
                answer,
                asked: Vec::new(),
            }
        }
    }

    impl Console for FakeConsole {
        fn is_interactive(&self) -> bool {
            self.interactive
        }

        fn confirm(&mut self, question: &str) -> io::Result<bool> {
            self.asked.push(question.to_owned());
            Ok(self.answer)
        }
    }

    #[test]
    fn a_person_at_a_terminal_is_asked_and_their_answer_decides() {
        let mut yes = FakeConsole::new(true, true);
        let mut no = FakeConsole::new(true, false);

        assert!(wants_apply_now(false, &mut yes, 2).expect("answer"));
        assert!(!wants_apply_now(false, &mut no, 2).expect("answer"));
        assert_eq!(yes.asked, ["Apply 2 staged changes?"]);
    }

    #[test]
    fn the_question_is_singular_for_one_change() {
        let mut console = FakeConsole::new(true, true);

        wants_apply_now(false, &mut console, 1).expect("answer");

        assert_eq!(console.asked, ["Apply 1 staged change?"]);
    }

    #[test]
    fn scripts_and_agents_are_never_asked() {
        let mut piped = FakeConsole::new(false, true);
        let mut json = FakeConsole::new(true, true);

        assert!(!wants_apply_now(false, &mut piped, 1).expect("answer"));
        assert!(!wants_apply_now(true, &mut json, 1).expect("answer"));
        assert!(piped.asked.is_empty() && json.asked.is_empty());
    }
}
