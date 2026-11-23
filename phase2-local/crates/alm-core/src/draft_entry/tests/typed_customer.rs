use super::*;

#[test]
fn typed_customer_is_preserved_when_retaining_the_draft() {
    let draft = ProvisionalDraft::new(DraftId::new(51), Vec::new());
    let facts = ActiveBillFacts::new(&[], "Typed Customer", 0.0, "");
    let requested = customer("cust-requested", "Requested Name");

    for choice in [DraftChoice::Cancel, DraftChoice::Continue] {
        let entry = DraftEntryAction::NewBill
            .begin_with_customer(Some((&draft, facts)), fresh_start(52, Some(requested)));
        let DraftEntry::AwaitingChoice(pending) = entry else {
            panic!("typed customer counts as work");
        };
        let form = match (choice, pending.resolve(choice)) {
            (DraftChoice::Cancel, DraftEntryOutcome::StayWithDraft(form))
            | (DraftChoice::Continue, DraftEntryOutcome::OpenExisting(form)) => form,
            _ => panic!("the choice must retain the existing form"),
        };
        assert!(std::ptr::eq(form.draft(), &draft));
        assert_eq!(form.customer(), None);
        assert_eq!(form.customer_name(), "Typed Customer");
    }
}
