use std::env;
use std::io::{self, BufRead, Write};
use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};
use std::process::ExitCode;
use std::str::FromStr;

use alm_store::{
    DraftCustomer, DraftId, DraftLine, ManagerDraftStore, ProductId, ProvisionalDraft,
};

#[path = "alm-draft-demo/launch.rs"]
mod launch;
#[path = "alm-draft-demo/review.rs"]
mod review;

struct CurrentDraft {
    id: NonZeroU64,
    draft: ProvisionalDraft,
    saved: bool,
}

struct Console<'a, R, W> {
    input: &'a mut R,
    output: &'a mut W,
}

impl<R: BufRead, W: Write> Console<'_, R, W> {
    fn read(&mut self, prompt: &str) -> io::Result<Option<String>> {
        writeln!(self.output, "{prompt}")?;
        self.output.flush()?;
        let mut line = String::new();
        if self.input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        Ok(Some(line.trim().to_owned()))
    }

    fn choice(&mut self, prompt: &str, expected: &str) -> io::Result<bool> {
        loop {
            match self.read(prompt)? {
                Some(value) if value == expected => return Ok(true),
                Some(_) => writeln!(self.output, "Please enter {expected}.")?,
                None => return Ok(false),
            }
        }
    }

    fn positive<T: FromStr>(&mut self, prompt: &str) -> io::Result<Option<T>> {
        loop {
            match self.read(prompt)? {
                Some(value) if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) => {
                    if let Ok(number) = value.parse() {
                        return Ok(Some(number));
                    }
                    writeln!(self.output, "Enter a positive whole number in range.")?;
                }
                Some(_) => writeln!(self.output, "Enter a positive whole number in range.")?,
                None => return Ok(None),
            }
        }
    }
}

fn next_id(session: &ManagerDraftStore) -> io::Result<NonZeroU64> {
    let mut id = NonZeroU64::MIN;
    loop {
        if session
            .reopen_draft(DraftId::new(id.get()))
            .map_err(io::Error::other)?
            .is_none()
        {
            return Ok(id);
        }
        id = id
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .ok_or_else(|| io::Error::other("No draft IDs are available."))?;
    }
}

fn new_draft<R: BufRead, W: Write>(
    console: &mut Console<'_, R, W>,
    session: &ManagerDraftStore,
) -> io::Result<Option<CurrentDraft>> {
    let id = next_id(session)?;
    if !console.choice("Customer (Synthetic Customer):", "Synthetic Customer")?
        || !console.choice("Item (1 = Synthetic Lime):", "1")?
    {
        return Ok(None);
    }
    let Some(quantity) = console.positive::<NonZeroU32>("Quantity (positive whole number):")?
    else {
        return Ok(None);
    };
    let draft = ProvisionalDraft::new(
        DraftId::new(id.get()),
        vec![DraftLine::new(ProductId::new(1), quantity)],
    )
    .with_customer(DraftCustomer::Typed("Synthetic Customer".to_owned()));
    Ok(Some(CurrentDraft {
        id,
        draft,
        saved: false,
    }))
}

fn run_menu(
    input: &mut impl BufRead,
    output: &mut impl Write,
    session: &mut ManagerDraftStore,
) -> io::Result<()> {
    let mut console = Console { input, output };
    let mut current: Option<CurrentDraft> = None;
    loop {
        let Some(command) = console.read("Command (new/save/open/edit/review/quit):")? else {
            return match current {
                Some(draft) if !draft.saved => Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("Input ended; draft {} was NOT SAVED.", draft.id),
                )),
                Some(_) | None => Ok(()),
            };
        };
        match command.as_str() {
            "new" | "open" | "quit" if current.as_ref().is_some_and(|draft| !draft.saved) => {
                writeln!(
                    console.output,
                    "Unsaved draft: use save before new, open, or quit."
                )?;
            }
            "new" => match new_draft(&mut console, session)? {
                Some(draft) => {
                    writeln!(console.output, "New draft {}: UNSAVED.", draft.id)?;
                    current = Some(draft);
                }
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Input ended; incomplete draft was not saved.",
                    ));
                }
            },
            "save" => match current.as_mut() {
                Some(draft) => match session.save_draft(draft.draft.clone()) {
                    Ok(()) => {
                        draft.saved = true;
                        writeln!(
                            console.output,
                            "Saved draft {}: PROVISIONAL — NOT POSTED.",
                            draft.id
                        )?;
                    }
                    Err(error) => writeln!(console.output, "Could not save draft: {error}")?,
                },
                None => writeln!(console.output, "No draft is open.")?,
            },
            "open" => {
                let Some(id) =
                    console.positive::<NonZeroU64>("Draft ID (positive whole number):")?
                else {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Input ended before draft ID.",
                    ));
                };
                match session.reopen_draft(DraftId::new(id.get())) {
                    Ok(Some(draft)) => {
                        writeln!(console.output, "Opened draft {id}: SAVED.")?;
                        current = Some(CurrentDraft {
                            id,
                            draft,
                            saved: true,
                        });
                    }
                    Ok(None) => writeln!(console.output, "Draft {id} was not found.")?,
                    Err(error) => writeln!(console.output, "Could not open draft: {error}")?,
                }
            }
            "edit" => {
                let Some(draft) = current.as_mut() else {
                    writeln!(console.output, "No draft is open.")?;
                    continue;
                };
                let line_count = draft.draft.lines().len();
                if line_count == 0 {
                    writeln!(console.output, "No ordinary lines to edit.")?;
                    continue;
                }
                let Some(line) = console.positive::<NonZeroUsize>("Line number (one-based):")?
                else {
                    continue;
                };
                if line.get() > line_count {
                    writeln!(console.output, "Line {line} was not found.")?;
                    continue;
                }
                let Some(quantity) =
                    console.positive::<NonZeroU32>("Quantity (positive whole number):")?
                else {
                    continue;
                };
                if draft
                    .draft
                    .replace_line_quantity(line, quantity)
                    .is_some_and(|previous| previous != quantity)
                {
                    draft.saved = false;
                    writeln!(console.output, "Edited draft {}: UNSAVED.", draft.id)?;
                }
            }
            "review" => match &current {
                Some(draft) => review::render(console.output, draft)?,
                None => writeln!(console.output, "No draft is open.")?,
            },
            "quit" => return Ok(()),
            _ => writeln!(
                console.output,
                "Unknown command. Use new, save, open, edit, review, or quit."
            )?,
        }
    }
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let (mut drafts, mode) = launch::open(&args)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(
        output,
        "ALM synthetic Manager demo — local drafts only; not authenticated."
    )?;
    writeln!(output, "{mode}")?;
    run_menu(&mut stdin.lock(), &mut output, &mut drafts)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
