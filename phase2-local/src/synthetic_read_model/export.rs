use std::fmt::Write;

use super::storage::{Error, Store};
use crate::Role;

fn money(minor: i64) -> String {
    let magnitude = minor.unsigned_abs();
    let sign = if minor < 0 { "-" } else { "" };
    let whole = magnitude / 100;
    let fraction = magnitude % 100;
    if fraction == 0 {
        format!("{sign}{whole}")
    } else if fraction.is_multiple_of(10) {
        format!("{sign}{whole}.{}", fraction / 10)
    } else {
        format!("{sign}{whole}.{fraction:02}")
    }
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into()
    }
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

enum Cell {
    Text(String),
    Number(String),
}

fn text(value: impl Into<String>) -> Cell {
    Cell::Text(value.into())
}
fn number(value: impl ToString) -> Cell {
    Cell::Number(value.to_string())
}

fn sheet(name: &str, headers: &[&str], rows: Vec<Vec<Cell>>) -> String {
    let mut result = format!("<Worksheet ss:Name=\"{}\"><Table>", xml(name));
    let header = headers.iter().map(|value| text(*value)).collect::<Vec<_>>();
    for row in std::iter::once(header).chain(rows) {
        result.push_str("<Row>");
        for cell in row {
            let (kind, value) = match cell {
                Cell::Text(v) => ("String", v),
                Cell::Number(v) => ("Number", v),
            };
            let _ = write!(
                result,
                "<Cell><Data ss:Type=\"{kind}\">{}</Data></Cell>",
                xml(&value)
            );
        }
        result.push_str("</Row>");
    }
    result.push_str("</Table></Worksheet>");
    result
}

impl Store {
    pub fn sales_csv(&self, role: Role) -> Result<String, Error> {
        let records = self.records(role)?;
        let mut sales = records.sales.iter().collect::<Vec<_>>();
        sales.sort_by_key(|sale| sale.date.parts());
        let mut rows = vec![
            "Date,Bill No,Customer,Mode,Item,Pack,Rate,Pkt,Amount,Bill Total,Paid,Status"
                .to_string(),
        ];
        for sale in sales {
            for (index, line) in sale.lines.iter().enumerate() {
                let row = [
                    sale.date.to_string(),
                    sale.number.to_string(),
                    sale.customer.clone(),
                    sale.mode.clone(),
                    line.display_item(),
                    format!("{}{}", line.pack, line.uom),
                    money(line.rate_minor),
                    line.quantity.to_string(),
                    money(line.amount_minor),
                    if index == 0 {
                        money(sale.total_minor)
                    } else {
                        String::new()
                    },
                    if index == 0 {
                        money(sale.paid_minor)
                    } else {
                        String::new()
                    },
                    if sale.cancelled {
                        "CANCELLED".into()
                    } else {
                        String::new()
                    },
                ];
                rows.push(
                    row.iter()
                        .map(|cell| csv(cell))
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
        }
        Ok(rows.join("\n"))
    }

    pub fn workbook(&self, role: Role) -> Result<String, Error> {
        let records = self.records(role)?;
        let mut output = "<?xml version=\"1.0\"?><?mso-application progid=\"Excel.Sheet\"?><Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\" xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\">".to_string();
        let products = records
            .products
            .iter()
            .map(|p| {
                vec![
                    text(&p.company),
                    text(&p.item),
                    text(&p.pack),
                    text(&p.uom),
                    number(p.packet_quantity),
                    number(money(p.mrp_minor)),
                    number(money(p.buy_minor)),
                    number(money(p.sell_minor)),
                    number(p.on_hand),
                ]
            })
            .collect();
        output.push_str(&sheet(
            "Products",
            &[
                "Company", "Item", "Pack", "UOM", "Pkt Qty", "MRP", "Buy", "Sell", "Stock",
            ],
            products,
        ));
        let customers = records
            .customers
            .iter()
            .map(|c| {
                vec![
                    text(&c.name),
                    text(&c.mobile),
                    text(&c.address),
                    number(money(c.balance_minor)),
                ]
            })
            .collect();
        output.push_str(&sheet(
            "Customers",
            &["Name", "Mobile", "Address", "Balance"],
            customers,
        ));
        let sales = records
            .sales
            .iter()
            .flat_map(|s| {
                s.lines.iter().enumerate().map(move |(i, l)| {
                    vec![
                        text(s.date.to_string()),
                        number(s.number),
                        text(&s.customer),
                        text(l.display_item()),
                        number(money(l.rate_minor)),
                        number(l.quantity),
                        number(money(l.amount_minor)),
                        if i == 0 {
                            number(money(s.total_minor))
                        } else {
                            text("")
                        },
                        if i == 0 {
                            number(money(s.paid_minor))
                        } else {
                            text("")
                        },
                        text(if s.cancelled { "CANCELLED" } else { "" }),
                    ]
                })
            })
            .collect();
        output.push_str(&sheet(
            "Sales",
            &[
                "Date", "No", "Customer", "Item", "Rate", "Pkt", "Amount", "Total", "Paid",
                "Status",
            ],
            sales,
        ));
        let stock = records
            .products
            .iter()
            .map(|p| {
                vec![
                    text(&p.item),
                    number(p.purchased),
                    number(p.sold),
                    number(p.on_hand),
                    number(p.reorder),
                ]
            })
            .collect();
        output.push_str(&sheet(
            "Stock",
            &["Item", "Purchased", "Sold", "On Hand", "Reorder"],
            stock,
        ));
        output.push_str("</Workbook>");
        Ok(output)
    }
}
