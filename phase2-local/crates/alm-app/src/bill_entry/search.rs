//! New Bill customer and item suggestions.
use alm_core::confirmation::owner_billing::BillingDirectory;

use crate::{Trial, views};

impl Trial {
    pub(crate) fn bill_customer_search(&self, q: &str) -> String {
        let found = self.directory.search_customers(self.drafting(), q);
        let options: Vec<_> = found
            .iter()
            .map(|c| [("customer", c.id), ("customer_q", c.name)])
            .collect();
        views::search_options(
            &found
                .iter()
                .zip(&options)
                .map(|(c, fill)| views::SearchOption {
                    fill,
                    label: c.name,
                    sub: "",
                    submit: Some("select"),
                })
                .collect::<Vec<_>>(),
        )
    }

    pub(crate) fn bill_item_search(&self, q: &str) -> String {
        let catalog = self.manager_catalog();
        let found = match &catalog {
            Some(catalog) => BillingDirectory::search_catalog(catalog, q),
            None => self.directory.search_products(self.drafting(), q),
        };
        let ids: Vec<String> = found.iter().map(|(id, _)| id.to_string()).collect();
        let fills: Vec<_> = found
            .iter()
            .zip(&ids)
            .map(|((_, name), id)| [("product", id.as_str()), ("product_q", *name)])
            .collect();
        views::search_options(
            &found
                .iter()
                .zip(&fills)
                .map(|((_, name), fill)| views::SearchOption {
                    fill,
                    label: name,
                    sub: "",
                    // The HTML's pick fills the line at once; here it adds the line.
                    submit: Some("add"),
                })
                .collect::<Vec<_>>(),
        )
    }
}
