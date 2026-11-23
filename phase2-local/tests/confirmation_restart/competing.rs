use super::{Client, ConfirmationError, DraftId, Money, OwnerRequest, Receipt, RequestId, Role};
use super::{
    DATABASE, Scratch, draft, effects, emit, open_sales, read_receipt, receipt_frame, send,
};
use crate::fixture;
use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

#[test]
fn competing_clients_with_identical_approval_receive_the_same_receipt() {
    compete(
        5,
        [(71, 1), (71, 1)],
        Dispatch {
            order: [0, 1],
            rejection: None,
        },
    );
}

#[test]
fn competing_clients_with_distinct_requests_for_one_draft_reject_the_second() {
    compete(
        5,
        [(71, 1), (72, 1)],
        Dispatch {
            order: [0, 1],
            rejection: Some(ConfirmationError::DraftAlreadyPosted),
        },
    );
}

#[test]
fn competing_clients_for_last_stock_post_only_the_first_delivered_draft() {
    for order in [[0, 1], [1, 0]] {
        compete(
            2,
            [(71, 1), (72, 2)],
            Dispatch {
                order,
                rejection: Some(ConfirmationError::InsufficientStock),
            },
        );
    }
}

struct Dispatch {
    order: [usize; 2],
    rejection: Option<ConfirmationError>,
}

fn compete(stock: u32, submissions: [(u64, u64); 2], dispatch: Dispatch) {
    // Given two explicit approvals from the same authority, at the same terms revision.
    let mut authority = fixture(stock);
    authority.register_draft(draft(2, 2)).unwrap();
    authority
        .set_unit_price(Money::from_minor_units(193))
        .unwrap();
    let approved = submissions.map(|(request, draft)| OwnerRequest {
        id: RequestId::new(request),
        review: authority.review(Role::Owner, DraftId::new(draft)).unwrap(),
    });
    let pristine = (stock, 0, 1, 0, 0);
    assert_eq!(effects(&authority), pristine);

    // When: release both fresh clients, then decode and independently read back both sends.
    let scratch = [Scratch::new(), Scratch::new()];
    let mut clients = scratch
        .each_ref()
        .map(|dir| Client::start("sqlite_compete", &dir.0));
    for client in &mut clients {
        client.ready();
        assert_eq!(effects(&authority), pristine);
    }
    for (client, approval) in clients.iter_mut().zip(&approved) {
        client.write_frame(&approval.encode());
    }
    let sent = [0, 1].map(|i| {
        let request = clients[i].sent();
        assert_eq!(request, approved[i]);
        open_sales(&scratch[i].0.join(DATABASE))
            .session(Role::Owner)
            .retry_pending(request.id, |id, review| {
                assert_eq!((id, review), (request.id, &request.review));
            })
            .expect("exact approval must be committed by SEND");
        assert_eq!(effects(&authority), pristine, "dispatch before both sends");
        request
    });

    // Then: both clients are waiting for replies. Only now dispatch serially, in chosen order.
    let [first, second] = dispatch.order;
    let winner = &approved[first];
    let receipt = Receipt {
        draft_id: winner.review.draft().id(),
        invoice: 1,
        quantity: 2,
        total: Money::from_minor_units(386),
        received: None,
        due: None,
        customer_balance: None,
    };
    let requests = BTreeMap::from([(winner.id, (winner.review.clone(), receipt.clone()))]);
    let posted = BTreeMap::from([(receipt.draft_id, winner.id)]);
    let second_result = dispatch.rejection.map_or_else(|| Ok(receipt.clone()), Err);
    let mut received = [String::new(), String::new()];
    for (i, expected) in [(first, Ok(receipt)), (second, second_result)] {
        let result = authority.confirm(Role::Owner, sent[i].id, &sent[i].review);
        assert_eq!(result, expected);
        assert_eq!(effects(&authority), (stock - 2, 386, 2, 1, 1));
        assert_eq!(authority.requests, requests);
        assert_eq!(authority.posted, posted);

        clients[i].write_frame(&response_frame(&result));
        received[i] = format!("RECEIVED {}", response_frame(&expected));
    }
    for (client, event) in clients.into_iter().zip(received) {
        assert_eq!(client.finish(), [event]);
    }
}

fn response_frame(result: &Result<Receipt, ConfirmationError>) -> String {
    match result {
        Ok(receipt) => receipt_frame(receipt),
        Err(error) => format!("REJECTED {error:?}"),
    }
}

pub(super) fn worker(database: &Path, input: &mut impl Iterator<Item = io::Result<String>>) {
    assert_eq!(
        fs::symlink_metadata(database).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    let mut sales = open_sales(database);
    emit("READY");
    let approval = OwnerRequest::decode(&input.next().expect("explicit Owner approval").unwrap());
    let result = sales
        .session(Role::Owner)
        .confirm_pending(approval.id, &approval.review, |id, review| {
            send(&OwnerRequest {
                id,
                review: review.clone(),
            });
            let reply = input
                .next()
                .expect("authority must reply to each client")
                .unwrap();
            match reply.as_str() {
                "REJECTED DraftAlreadyPosted" => Err(ConfirmationError::DraftAlreadyPosted),
                "REJECTED InsufficientStock" => Err(ConfirmationError::InsufficientStock),
                frame => Ok(read_receipt(frame).expect("expected an authority response")),
            }
        })
        .expect("persist the Owner approval");
    emit(&format!("RECEIVED {}", response_frame(&result)));
}
