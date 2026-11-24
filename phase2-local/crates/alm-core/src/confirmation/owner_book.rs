//! The authority's state as Owner book rows, and the device's read-only rebuild from them.
//! Reviews, idempotency records and registered drafts are the authority's own and are not
//! part of the book: a rebuilt view prices and previews, it never posts.
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use super::crate_templates::CrateTemplateBook;
use super::customer_opening::StoredOpening;
use super::manual_cashbook::ManualCashBook;
use super::preview::PreviewPayment;
use super::purchase_orders::PurchaseBook;
use super::{
    Catalog, ConfirmationError, ConfirmationFixture, CreationKey, CustomerReceiptMetadata,
    GodownCases, GstBillId, ManualGodownRow, PostedLineItem, ProposedPayment, Receipt, ReceiptMode,
    Review,
};
use super::{CheckoutTerms, LineTerms, ProductTerms};
use crate::projection::book::{
    BookCheckout, BookCrateRow, BookCustomer, BookCustomerRef, BookPayment, BookReceipt, BookSale,
    BookSaleLine, BookSettings, OwnerBook,
};
use crate::{
    CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role, SaleUnit,
};

fn customer_ref(customer: &DraftCustomer) -> BookCustomerRef {
    match customer {
        DraftCustomer::Unassigned => BookCustomerRef::Unassigned,
        DraftCustomer::Typed(name) => BookCustomerRef::Typed(name.clone()),
        DraftCustomer::Selected { id, name } => BookCustomerRef::Selected {
            id: id.value().to_owned(),
            name: name.clone(),
        },
    }
}

fn draft_customer(customer: &BookCustomerRef) -> DraftCustomer {
    match customer {
        BookCustomerRef::Unassigned => DraftCustomer::Unassigned,
        BookCustomerRef::Typed(name) => DraftCustomer::Typed(name.clone()),
        BookCustomerRef::Selected { id, name } => DraftCustomer::Selected {
            id: DraftCustomerId::new(id.clone()),
            name: name.clone(),
        },
    }
}

const fn payment(value: ProposedPayment) -> BookPayment {
    match value {
        ProposedPayment::Legacy => BookPayment::Legacy,
        ProposedPayment::Named(m) => BookPayment::Named(m),
        ProposedPayment::WalkInCash(m) => BookPayment::WalkInCash(m),
    }
}

const fn proposed(value: BookPayment) -> ProposedPayment {
    match value {
        BookPayment::Legacy => ProposedPayment::Legacy,
        BookPayment::Named(m) => ProposedPayment::Named(m),
        BookPayment::WalkInCash(m) => ProposedPayment::WalkInCash(m),
    }
}

fn nonzero(value: u32) -> Result<NonZeroU32, ConfirmationError> {
    NonZeroU32::new(value).ok_or(ConfirmationError::InvalidDraft)
}

const fn unit(piece: bool) -> SaleUnit {
    if piece {
        SaleUnit::Piece
    } else {
        SaleUnit::Case
    }
}

fn crate_row(row: &BookCrateRow) -> Result<CrateRow, ConfirmationError> {
    let components = row
        .components
        .iter()
        .map(|&(product, quantity, piece)| {
            Ok(CrateComponent::new(
                ProductId::new(product),
                nonzero(quantity)?,
                unit(piece),
            ))
        })
        .collect::<Result<_, ConfirmationError>>()?;
    let base = CrateRow::new(nonzero(row.quantity)?, components);
    Ok(match &row.price {
        Some((name, rate)) => base.with_price(name.clone(), *rate),
        None => base,
    })
}

fn checkout(c: &BookCheckout) -> Result<CheckoutTerms, ConfirmationError> {
    Ok(CheckoutTerms {
        lines: c
            .lines
            .iter()
            .map(|&(piece, pack, piece_price, case_price)| {
                Ok(LineTerms {
                    unit: unit(piece),
                    product: ProductTerms {
                        pack: nonzero(pack)?,
                        piece_price,
                        case_price,
                    },
                })
            })
            .collect::<Result<_, ConfirmationError>>()?,
        loading_rate: c.loading_rate,
    })
}

impl ConfirmationFixture {
    /// The Owner's book rows of this state.
    ///
    /// # Errors
    /// Denies Managers.
    pub fn owner_book(
        &self,
        role: Role,
        cashbook: &ManualCashBook,
    ) -> Result<OwnerBook, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut sales = Vec::new();
        for (id, key) in &self.first_posts {
            let (review, receipt, edits) = self
                .current_sales
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            let (cancelled, epoch) = self.lifecycle.get(id).copied().unwrap_or((false, 0));
            let (cost, line_units, allocations) = self.catalog.book_sale_costs(id.value())?;
            sales.push(BookSale {
                draft: id.value(),
                invoice: receipt.invoice,
                customer: customer_ref(review.draft.customer()),
                date: review.draft.bill_date(),
                loading_rate: review.draft.loading_rate(),
                lines: review
                    .draft
                    .lines()
                    .iter()
                    .map(|line| BookSaleLine {
                        product: line.product_id().value(),
                        quantity: line.quantity().get(),
                    })
                    .collect(),
                prices: review.prices.clone(),
                crate_rows: review
                    .draft
                    .crate_rows()
                    .iter()
                    .map(|row| BookCrateRow {
                        quantity: row.quantity().get(),
                        components: row
                            .components()
                            .iter()
                            .map(|c| {
                                (
                                    c.product.value(),
                                    c.quantity.get(),
                                    c.unit == SaleUnit::Piece,
                                )
                            })
                            .collect(),
                        price: row.name().zip(row.rate()).map(|(n, r)| (n.to_owned(), r)),
                    })
                    .collect(),
                checkout: review.checkout.as_ref().map(|c| BookCheckout {
                    lines: c
                        .lines
                        .iter()
                        .map(|l| {
                            (
                                l.unit == SaleUnit::Piece,
                                l.product.pack.get(),
                                l.product.piece_price,
                                l.product.case_price,
                            )
                        })
                        .collect(),
                    loading_rate: c.loading_rate,
                }),
                crate_packs: review
                    .crate_packs
                    .as_ref()
                    .map(|packs| packs.iter().map(|p| p.get()).collect()),
                payment: payment(review.payment),
                online: review.payment_mode == ReceiptMode::Online,
                quantity: receipt.quantity,
                total: receipt.total,
                received: receipt.received,
                due: receipt.due,
                customer_balance: receipt.customer_balance,
                edits: *edits,
                cancelled,
                epoch,
                creation_key: key.map(CreationKey::value),
                items: self
                    .posted_sale_items
                    .get(id)
                    .map(|items| {
                        items
                            .iter()
                            .map(|i| (i.product_id.value(), i.item.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                cost,
                line_units,
                allocations,
            });
        }
        let ids: BTreeSet<&String> = self
            .registered_customers
            .iter()
            .chain(self.customer_balances.keys())
            .chain(self.customer_openings.keys())
            .collect();
        let customers = ids
            .into_iter()
            .map(|id| BookCustomer {
                id: id.clone(),
                registered: self.registered_customers.contains(id),
                balance: self
                    .customer_balances
                    .get(id)
                    .map(|b| Money::from_minor_units(*b)),
                opening: self.customer_openings.get(id).map(|o| (o.amount, o.date)),
            })
            .collect();
        let (cash_days, next_cash) = cashbook.book();
        Ok(OwnerBook {
            settings: BookSettings {
                default_product: self.product.value(),
                reorder_default: self.product_reorder_default,
                loading_rate: self.loading_rate,
                crate_rate: self.crate_rate,
                next_invoice: self.next_invoice,
                movement: self.movement_epoch,
                ledger: Money::from_minor_units(self.ledger),
                received_ledger: Money::from_minor_units(self.received_ledger),
                next_cash,
            },
            products: self.catalog.book_products(),
            checkout_terms: self
                .checkout_products
                .iter()
                .map(|(p, t)| (*p, t.pack.get(), t.piece_price, t.case_price))
                .collect(),
            sales,
            receipts: self
                .preview_payments
                .iter()
                .map(|p| BookReceipt {
                    customer: p.customer_id.clone(),
                    amount: p.amount,
                    date: p.date,
                    after_invoice: p.after_invoice,
                    online: p.mode.map(|m| m == ReceiptMode::Online),
                    selected: p
                        .metadata
                        .as_ref()
                        .map(|m| (m.selected_name.clone(), m.creation_key.value())),
                })
                .collect(),
            customers,
            purchasing: self.purchases.book()?,
            crates: self.crate_templates.book(),
            godown: self
                .manual_godown
                .values()
                .map(|r| crate::projection::book::BookGodownRow {
                    item: r.item.clone(),
                    cases: r.cases.get(),
                    note: r.note.clone(),
                })
                .collect(),
            gst_bills: self.gst_bills.values().cloned().collect(),
            gst_deleted: self.gst_deleted.iter().map(|id| id.0).collect(),
            cash_days,
            activity: Vec::new(),
            legacy: crate::legacy::import::LegacyImports::default(),
        })
    }

    /// A read-only view rebuilt from the Owner's book rows: every Owner screen, the priced
    /// review and the edit and receipt checks read it. It holds no idempotency records and no
    /// registered drafts, so it never stands in for the authority.
    ///
    /// # Errors
    /// Rejects inconsistent rows.
    pub fn from_owner_book(book: &OwnerBook) -> Result<(Self, ManualCashBook), ConfirmationError> {
        let s = &book.settings;
        let mut fixture = Self::new(
            ProductId::new(s.default_product),
            0,
            Money::from_minor_units(0),
        )?;
        fixture.catalog = Catalog::from_book(&book.products, &book.sales)?;
        fixture.product_reorder_default = s.reorder_default;
        fixture.loading_rate = s.loading_rate;
        fixture.crate_rate = s.crate_rate;
        fixture.next_invoice = s.next_invoice;
        fixture.movement_epoch = s.movement;
        fixture.ledger = s.ledger.minor_units();
        fixture.received_ledger = s.received_ledger.minor_units();
        for &(product, pack, piece_price, case_price) in &book.checkout_terms {
            fixture.checkout_products.insert(
                product,
                ProductTerms {
                    pack: NonZeroU32::new(pack).ok_or(ConfirmationError::InvalidDraft)?,
                    piece_price,
                    case_price,
                },
            );
        }
        for sale in &book.sales {
            let id = DraftId::new(sale.draft);
            let lines = sale
                .lines
                .iter()
                .map(|l| {
                    Ok(DraftLine::new(
                        ProductId::new(l.product),
                        NonZeroU32::new(l.quantity).ok_or(ConfirmationError::InvalidDraft)?,
                    ))
                })
                .collect::<Result<_, ConfirmationError>>()?;
            let mut draft = ProvisionalDraft::new(id, lines)
                .with_customer(draft_customer(&sale.customer))
                .with_bill_date(sale.date)
                .with_loading_rate(sale.loading_rate);
            for row in &sale.crate_rows {
                draft.add_crate_row(crate_row(row)?);
            }
            let review = Review {
                draft,
                draft_revision: 0,
                terms_revision: 0,
                prices: sale.prices.clone(),
                payment: proposed(sale.payment),
                payment_mode: if sale.online {
                    ReceiptMode::Online
                } else {
                    ReceiptMode::Cash
                },
                checkout: sale.checkout.as_ref().map(checkout).transpose()?,
                crate_packs: sale
                    .crate_packs
                    .as_ref()
                    .map(|packs| packs.iter().map(|p| nonzero(*p)).collect())
                    .transpose()?,
            };
            let receipt = Receipt {
                draft_id: id,
                invoice: sale.invoice,
                quantity: sale.quantity,
                total: sale.total,
                received: sale.received,
                due: sale.due,
                customer_balance: sale.customer_balance,
            };
            fixture
                .current_sales
                .insert(id, (review, receipt, sale.edits));
            fixture.lifecycle.insert(id, (sale.cancelled, sale.epoch));
            fixture
                .first_posts
                .push((id, sale.creation_key.map(CreationKey::new)));
            fixture.posted_sale_items.insert(
                id,
                sale.items
                    .iter()
                    .map(|(product, item)| PostedLineItem {
                        product_id: ProductId::new(*product),
                        item: item.clone(),
                    })
                    .collect(),
            );
        }
        for c in &book.customers {
            if c.registered {
                fixture.registered_customers.insert(c.id.clone());
            }
            if let Some(balance) = c.balance {
                fixture
                    .customer_balances
                    .insert(c.id.clone(), balance.minor_units());
            }
            if let Some((amount, date)) = c.opening {
                fixture
                    .customer_openings
                    .insert(c.id.clone(), StoredOpening { amount, date });
            }
        }
        fixture.preview_payments = book
            .receipts
            .iter()
            .map(|r| PreviewPayment {
                customer_id: r.customer.clone(),
                amount: r.amount,
                date: r.date,
                after_invoice: r.after_invoice,
                mode: r.online.map(|online| {
                    if online {
                        ReceiptMode::Online
                    } else {
                        ReceiptMode::Cash
                    }
                }),
                metadata: r
                    .selected
                    .as_ref()
                    .map(|(name, key)| CustomerReceiptMetadata {
                        selected_name: name.clone(),
                        creation_key: CreationKey::new(*key),
                    }),
            })
            .collect();
        fixture.purchases = PurchaseBook::from_book(&book.purchasing)?;
        fixture.crate_templates = CrateTemplateBook::from_book(&book.crates)?;
        fixture.manual_godown = book
            .godown
            .iter()
            .map(|r| {
                (
                    r.item.to_lowercase(),
                    ManualGodownRow {
                        item: r.item.clone(),
                        cases: GodownCases::new(r.cases),
                        note: r.note.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        fixture.gst_bills = book.gst_bills.iter().map(|b| (b.id, b.clone())).collect();
        fixture.gst_deleted = book.gst_deleted.iter().map(|id| GstBillId(*id)).collect();
        let cashbook = ManualCashBook::from_book(&book.cash_days, s.next_cash);
        Ok((fixture, cashbook))
    }
}
