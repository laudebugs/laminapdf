//! Turn an error's code into what your app does next: ask for a password, ask again, or tell the
//! person the file isn't a PDF. Any other code is returned as the error, so a code your app
//! doesn't know yet isn't swallowed.
//!
//! ```sh
//! cargo run -p laminapdf --example react_to_error_codes -- in.pdf [password]
//! ```

use std::error::Error;

use laminapdf::{Document, ErrorCode, OpenOptions, Warning, WarningCode};

/// What the app does after trying to open a file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NextStep {
    Show { pages: u32, warnings: Vec<WarningCode> },
    AskForPassword,
    AskAgain,
    NotAPdf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or("usage: react_to_error_codes in.pdf [password]")?;
    let password = args.next();
    let next = open_or_ask(std::fs::read(input)?, password.as_deref())?;
    println!("{next:?}");
    Ok(())
}

/// Opens `bytes`, with `password` if one was given, and says what to do next.
pub(crate) fn open_or_ask(bytes: Vec<u8>, password: Option<&str>) -> Result<NextStep, laminapdf::Error> {
    let options = match password {
        Some(password) => OpenOptions::new().with_password(password),
        None => OpenOptions::new(),
    };
    match Document::open_bytes(bytes, options) {
        Ok(doc) => {
            // Warnings mean the file opened but needed repairs; their codes say which.
            let warnings = doc
                .warnings()
                .next_batch(u32::MAX)
                .iter()
                .map(Warning::warning_code)
                .collect();
            Ok(NextStep::Show {
                pages: doc.page_count()?,
                warnings,
            })
        }
        Err(e) => match e.error_code() {
            ErrorCode::PasswordRequired => Ok(NextStep::AskForPassword),
            ErrorCode::WrongPassword => Ok(NextStep::AskAgain),
            ErrorCode::NotPdf => Ok(NextStep::NotAPdf),
            _ => Err(e),
        },
    }
}
