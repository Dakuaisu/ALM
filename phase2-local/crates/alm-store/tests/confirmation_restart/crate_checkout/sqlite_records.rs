use std::path::Path;

use super::sqlite_list::{open_read_only, sqlite_list};

pub(super) fn assert_cost_free_records(database: &Path) {
    let sql = "
        SELECT m.name || ':' || (
            SELECT group_concat(name, ',') FROM (
                SELECT name FROM pragma_table_xinfo(m.name) ORDER BY cid
            )
        ) FROM sqlite_schema AS m WHERE m.type IN ('table', 'view') ORDER BY m.name;
        SELECT 'draft', * FROM provisional_drafts ORDER BY id;
        SELECT 'customer', * FROM provisional_draft_customers ORDER BY draft_id;
        SELECT 'date', * FROM provisional_draft_dates ORDER BY draft_id;
        SELECT 'lines', count(*) FROM provisional_draft_lines;
        SELECT 'crate', * FROM provisional_draft_crates ORDER BY draft_id;
        SELECT 'pending', * FROM pending_owner_confirmations ORDER BY request_id;
    ";
    let conn = open_read_only(database).expect("open raw client SQLite records");
    let output = sqlite_list(&conn, sql).expect("query raw client SQLite records");
    assert_eq!(
        output,
        concat!(
            "pending_owner_confirmations:request_id,snapshot\n",
            "provisional_draft_crates:draft_id,definition\n",
            "provisional_draft_customers:draft_id,kind,customer_id,name\n",
            "provisional_draft_dates:draft_id,bill_date\n",
            "provisional_draft_lines:draft_id,position,product_id,quantity\n",
            "provisional_draft_loading:draft_id,rate\n",
            "provisional_drafts:id\n",
            "draft|43\n",
            "customer|43|2|crate-buyer|Synthetic\n",
            "date|43|2024-02-29\n",
            "lines|0\n",
            "crate|43|2;2,9,0;1,2,1\n",
            "pending|987|6|2024-02-29|5|43|1|5|2,63726174652D6275796572,53796E746865746963|",
            "2;2,9,0;1,2,1|450|13|6,1|100\n",
        )
    );
}
