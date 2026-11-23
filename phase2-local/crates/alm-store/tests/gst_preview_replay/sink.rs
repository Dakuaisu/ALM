use std::time::Duration;

use alm_store::confirmation::GstBillInvoice;

use super::markup;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    Print,
    SavePdf,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Event {
    Render(Action),
    Open(Action, &'static str, &'static str, &'static str),
    Warning(Action, &'static str, &'static str),
    Write(Action),
    Close(Action),
}

pub struct Document {
    pub html: String,
    pub print_after_load: Duration,
}

#[derive(Default)]
pub struct Sink {
    pub events: Vec<Event>,
    pub documents: Vec<Document>,
}

impl Sink {
    pub fn invoke(&mut self, action: Action, invoice: &GstBillInvoice, popup: bool) {
        let body = markup::render(invoice);
        let title = markup::title(invoice);
        self.events.push(Event::Render(action));
        self.events
            .push(Event::Open(action, "", "_p", "width=840,height=920"));
        if !popup {
            self.events.push(Event::Warning(
                action,
                "Pop-up blocked — allow pop-ups for this page to print",
                "warn",
            ));
            return;
        }
        self.events.push(Event::Write(action));
        self.documents.push(Document {
            html: format!("<html><head><title>{title}</title><style>@page{{margin:0}}body{{margin:0;padding:10mm}}</style></head><body style=\"margin:0\">{body}<script type=\"application/x-alm-inert\">window.onload=function(){{setTimeout(function(){{window.print();}},350);}}</script></body></html>"),
            print_after_load: Duration::from_millis(350),
        });
        self.events.push(Event::Close(action));
    }
}
