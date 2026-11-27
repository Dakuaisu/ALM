//! New Bill customer and item suggestions.
use alm_core::confirmation::owner_billing::BillingDirectory;

use crate::{Trial, views};

impl Trial {
    pub(crate) fn bill_customer_search(&self, q: &str) -> String {
        let names = self.manager_customers();
        let found: Vec<(String, String)> = match &names {
            Some(names) => BillingDirectory::search_customer_names(names, q)
                .into_iter()
                .map(|c| (c.id.clone(), c.name.clone()))
                .collect(),
            None => self
                .directory
                .search_customers(self.drafting(), q)
                .into_iter()
                .map(|c| (c.id, c.name))
                .collect(),
        };
        let options: Vec<_> = found
            .iter()
            .map(|(id, name)| [("customer", id.as_str()), ("customer_q", name.as_str())])
            .collect();
        views::search_options(
            &found
                .iter()
                .zip(&options)
                .map(|((_, name), fill)| views::SearchOption {
                    fill,
                    label: name,
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
