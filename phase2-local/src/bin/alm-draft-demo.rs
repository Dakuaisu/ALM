use std::env;
use std::io::{self, BufRead, Write};
use std::num::{NonZeroU32, NonZeroU64};
use std::path::Path;
use std::process::ExitCode;
use std::str::FromStr;

use alm_phase2_local::{
    DraftCustomer, DraftId, DraftLine, LocalSales, Money, ProductId, ProvisionalDraft, Role,
    SalesSession,
};

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

fn next_id(session: &SalesSession<'_>) -> io::Result<NonZeroU64> {
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
    session: &SalesSession<'_>,
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

fn review(output: &mut impl Write, current: &CurrentDraft) -> io::Result<()> {
    writeln!(output, "PROVISIONAL — NOT POSTED")?;
    writeln!(output, "Draft ID: {}", current.id)?;
    let state = if current.saved { "SAVED" } else { "UNSAVED" };
    writeln!(output, "State: {state}")?;
    let customer = match current.draft.customer() {
        DraftCustomer::Unassigned => "Unassigned",
        DraftCustomer::Typed(name) | DraftCustomer::Selected { name, .. } => name,
    };
    writeln!(output, "Customer: {customer}")?;
    for line in current.draft.lines() {
        let item = if line.product_id() == ProductId::new(1) {
            "Synthetic Lime"
        } else {
            "Unrecognized item"
        };
        writeln!(output, "Item: {item} | Quantity: {}", line.quantity())?;
    }
    Ok(())
}

fn run_menu(
    input: &mut impl BufRead,
    output: &mut impl Write,
    session: &mut SalesSession<'_>,
) -> io::Result<()> {
    let mut console = Console { input, output };
    let mut current: Option<CurrentDraft> = None;
    loop {
        let Some(command) = console.read("Command (new/save/open/review/quit):")? else {
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
            "review" => match &current {
                Some(draft) => review(console.output, draft)?,
                None => writeln!(console.output, "No draft is open.")?,
            },
            "quit" => return Ok(()),
            _ => writeln!(
                console.output,
                "Unknown command. Use new, save, open, review, or quit."
            )?,
        }
    }
}

fn run() -> io::Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let usage = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: alm-draft-demo --db /absolute/path/to/synthetic.sqlite",
        )
    };
    let [flag, path] = args.as_slice() else {
        return Err(usage());
    };
    if flag != "--db" || !Path::new(path).is_absolute() {
        return Err(usage());
    }
    let mut sales = LocalSales::open_sqlite(Path::new(path), Money::from_minor_units(0))
        .map_err(io::Error::other)?;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(
        output,
        "ALM synthetic Manager demo — local drafts only; not authenticated."
    )?;
    run_menu(
        &mut stdin.lock(),
        &mut output,
        &mut sales.session(Role::Manager),
    )
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use alm_phase2_local::{DraftCustomer, DraftId, LocalSales, Money, ProductId, Role};

    #[test]
    fn saves_a_synthetic_draft_when_explicitly_requested() {
        let mut input = Cursor::new(b"new\nSynthetic Customer\n1\n3\nsave\nreview\nquit\n");
        let mut output = Vec::new();
        let mut sales = LocalSales::new(Money::from_minor_units(0));

        super::run_menu(&mut input, &mut output, &mut sales.session(Role::Manager)).unwrap();

        let draft = sales
            .session(Role::Manager)
            .reopen_draft(DraftId::new(1))
            .unwrap()
            .unwrap();
        assert_eq!(
            draft.customer(),
            &DraftCustomer::Typed("Synthetic Customer".into())
        );
        assert_eq!(draft.lines()[0].product_id(), ProductId::new(1));
        assert_eq!(draft.lines()[0].quantity().get(), 3);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("PROVISIONAL — NOT POSTED")
        );
    }
}
