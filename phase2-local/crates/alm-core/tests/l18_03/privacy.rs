use alm_core::Role;
use alm_core::confirmation::ConfirmationError;

use super::{SALE, fixture};

#[derive(Clone, Copy)]
enum Target {
    Figure(usize),
    Ordinary,
    Body,
}

#[derive(Debug, Eq, PartialEq)]
struct Event {
    propagation_stopped: bool,
    default_prevented: bool,
    card_opened: bool,
}

struct Privacy {
    enabled: bool,
    revealed: [bool; 2],
}

impl Privacy {
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.revealed.fill(false);
    }

    fn click(&mut self, target: Target) -> Event {
        let captured = match target {
            Target::Figure(index) if self.enabled => {
                self.revealed[index] = !self.revealed[index];
                true
            }
            Target::Figure(_) | Target::Ordinary | Target::Body => false,
        };
        Event {
            propagation_stopped: captured,
            default_prevented: captured,
            card_opened: !captured && matches!(target, Target::Figure(_)),
        }
    }

    fn visible(&self, index: usize) -> bool {
        !self.enabled || self.revealed[index]
    }
}

#[test]
fn capture_peeks_one_figure_consumes_event_and_remasks() {
    let (fixture, buyer, _) = fixture();
    let owner_bill = fixture.current_sale(Role::Owner, SALE).unwrap();
    let owner_ledger = fixture.customer_ledger(Role::Owner, &buyer).unwrap();
    assert_eq!(
        fixture.current_sale(Role::Manager, SALE),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        fixture.customer_ledger(Role::Manager, &buyer),
        Err(ConfirmationError::AccessDenied)
    );

    let mut privacy = Privacy {
        enabled: false,
        revealed: [false; 2],
    };
    assert!(privacy.visible(0) && privacy.visible(1));
    assert_eq!(
        privacy.click(Target::Figure(0)),
        Event {
            propagation_stopped: false,
            default_prevented: false,
            card_opened: true
        }
    );
    privacy.set_enabled(true);
    assert!(!privacy.visible(0) && !privacy.visible(1));
    assert_eq!(
        privacy.click(Target::Ordinary),
        Event {
            propagation_stopped: false,
            default_prevented: false,
            card_opened: false
        }
    );
    assert_eq!(
        privacy.click(Target::Body),
        Event {
            propagation_stopped: false,
            default_prevented: false,
            card_opened: false
        }
    );
    assert_eq!(
        privacy.click(Target::Figure(0)),
        Event {
            propagation_stopped: true,
            default_prevented: true,
            card_opened: false
        }
    );
    assert!(privacy.visible(0) && !privacy.visible(1));
    assert_eq!(
        fixture
            .customer_ledger(Role::Owner, &buyer)
            .unwrap()
            .closing,
        owner_ledger.closing
    );
    assert_eq!(
        fixture.current_sale(Role::Manager, SALE),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        fixture.customer_ledger(Role::Manager, &buyer),
        Err(ConfirmationError::AccessDenied)
    );
    privacy.click(Target::Figure(0));
    assert!(!privacy.visible(0));
    privacy.click(Target::Figure(1));
    assert!(!privacy.visible(0) && privacy.visible(1));
    privacy.set_enabled(false);
    assert!(privacy.visible(0) && privacy.visible(1));
    privacy.set_enabled(true);
    assert!(!privacy.visible(0) && !privacy.visible(1));
    assert_eq!(fixture.current_sale(Role::Owner, SALE).unwrap(), owner_bill);
    assert_eq!(
        fixture.customer_ledger(Role::Owner, &buyer).unwrap(),
        owner_ledger
    );
}
