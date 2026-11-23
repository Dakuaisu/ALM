#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormKind {
    EditBill,
    ReceivePayment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub dirty: bool,
    pub customer: String,
    pub amount: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Layer {
    Parent,
    Form(Form),
    Discard,
    Confirm,
}

#[derive(Clone, Copy, Debug)]
pub enum Dismiss {
    Header,
    KeepEditing,
    Cancel,
    Escape,
    Overlay,
    Inside,
}

#[derive(Clone, Copy, Debug)]
pub enum BodyEvent {
    Input,
    Change,
    Button,
    ButtonChild,
    Plain,
    Footer,
}

pub struct Replay {
    pub layers: Vec<Layer>,
}

impl Replay {
    pub fn new(kind: FormKind) -> Self {
        Self {
            layers: vec![
                Layer::Parent,
                Layer::Form(Form {
                    kind,
                    dirty: false,
                    customer: "Synthetic customer".into(),
                    amount: "100".into(),
                }),
            ],
        }
    }

    pub fn form(&self) -> &Form {
        match &self.layers[1] {
            Layer::Form(form) => form,
            Layer::Parent | Layer::Discard | Layer::Confirm => panic!("form missing"),
        }
    }

    pub fn event(&mut self, event: BodyEvent) {
        let Layer::Form(form) = &mut self.layers[1] else {
            panic!("form missing");
        };
        match event {
            BodyEvent::Input | BodyEvent::Change | BodyEvent::Button | BodyEvent::ButtonChild => {
                form.dirty = true;
            }
            BodyEvent::Plain | BodyEvent::Footer => {}
        }
    }

    pub fn dismiss(&mut self, how: Dismiss) {
        match how {
            Dismiss::Inside => (),
            Dismiss::Header | Dismiss::KeepEditing | Dismiss::Cancel => {
                self.layers.pop();
            }
            Dismiss::Escape | Dismiss::Overlay => {
                if matches!(
                    self.layers.last(),
                    Some(Layer::Form(Form { dirty: true, .. }))
                ) {
                    self.layers.push(Layer::Discard);
                } else {
                    self.layers.pop();
                }
            }
        }
    }

    pub fn discard(&mut self) {
        assert_eq!(self.layers.pop(), Some(Layer::Discard));
        assert!(matches!(self.layers.pop(), Some(Layer::Form(_))));
    }

    pub fn confirm_yes(&mut self, callback: impl FnOnce(&Self)) {
        assert_eq!(self.layers.pop(), Some(Layer::Confirm));
        callback(self);
    }
}
