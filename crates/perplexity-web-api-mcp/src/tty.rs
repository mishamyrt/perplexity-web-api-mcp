use std::io::{IsTerminal, stdin, stdout};

pub(crate) fn is_interactive() -> bool {
    stdin().is_terminal() && stdout().is_terminal()
}
