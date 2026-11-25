-- Local development schema, continued: shop-scale work without quadratic steps (20,000 bills,
-- a legacy file of that size). Same results as before; checked by the differential and
-- conformance suites and timed by the alm-sync scale test.
--
-- * `import_legacy` collected manifest entries by concatenating a JSON array (a copy per
--   record) and numbered each new record by counting that kind's records again: both are now
--   linear (entries appended to an array, counts per kind kept as it goes).
-- * `crate_rows` (the pull's crate and mixed bills) rewrote the whole Sales, sale-cost and book
--   arrays once per crate bill: one pass now.
-- * A customer's balance scanned every bill and receipt (no index on the customer), once per
--   bill in the pull and after every posted bill: bills and receipts are indexed by customer,
--   and the pull computes each customer's balance once.
-- * The owner book numbered every lot of every product again for each bill's allocations:
--   the lots are numbered once.
-- * `import_legacy` looked up each record by (kind, legacy ID) through a cached generic plan.
--   PL/pgSQL builds that plan after a few calls, while `legacy_records` still looks empty, and
--   keeps it for the rest of the call: a sequential scan per record as the table grows (66 s
--   instead of 4 s for a 20,000-bill file whenever the table's statistics were small, e.g. after
--   a vacuum). The function now plans each statement with the table's current size.
-- * Each command took one transaction lock per request and draft ID (migration
--   20261128040000 bounds them).

create index sales_customer_idx on public.sales (customer_id);
create index receipts_customer_idx on public.receipts (customer_id);

create or replace function public.import_legacy(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
set plan_cache_mode = force_custom_plan
as $$
declare
    v_done jsonb;
    v_id bigint;
    v_entries jsonb[] := '{}';
    v_counts jsonb;
    v_q jsonb;
    v_r jsonb;
    v_ref jsonb;
    v_missing jsonb;
    v_existing public.legacy_records%rowtype;
    v_aid text;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'import_legacy', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if jsonb_typeof(p_terms -> 'digest') is distinct from 'string'
        or jsonb_typeof(p_terms -> 'version') is distinct from 'string'
        or jsonb_typeof(p_terms -> 'records') is distinct from 'array'
        or jsonb_typeof(p_terms -> 'quarantine') is distinct from 'array' then
        perform alm_private.invalid('plan');
    end if;
    -- The source file must come with the plan and hash to its digest.
    if jsonb_typeof(p_terms -> 'source') is distinct from 'string'
        or p_terms ->> 'source' = '' then
        raise exception 'invalid_terms: no source file' using errcode = 'ALM05';
    end if;
    if encode(extensions.digest(convert_to(p_terms ->> 'source', 'UTF8'), 'sha256'), 'hex')
            <> p_terms ->> 'digest' then
        raise exception 'invalid_terms: source digest' using errcode = 'ALM05';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('legacy_import', 0));
    insert into public.legacy_sources (digest, source)
        values (p_terms ->> 'digest', p_terms ->> 'source')
        on conflict (digest) do nothing;
    select id into v_id from public.legacy_manifests
        where digest = p_terms ->> 'digest' and version = p_terms ->> 'version';
    if found then
        return alm_private.record(p_request_id, 'import_legacy', p_terms,
            jsonb_build_object('created', v_id));
    end if;
    v_id := coalesce((select max(id) from public.legacy_manifests), 0) + 1;
    v_counts := coalesce((select jsonb_object_agg(kind, n) from (select kind, count(*) as n
        from public.legacy_records group by kind) c), '{}');
    for v_q in select e from jsonb_array_elements(p_terms -> 'quarantine') with ordinality t(e, n)
        order by n
    loop
        v_entries := array_append(v_entries, jsonb_build_object(
            'kind', v_q ->> 'kind', 'legacy_id', v_q ->> 'legacy_id', 'authority_id', null,
            'status', 'quarantined', 'reason', v_q ->> 'reason',
            'source', coalesce(v_q ->> 'source', '')));
    end loop;
    for v_r in select e from jsonb_array_elements(p_terms -> 'records') with ordinality t(e, n)
        order by n
    loop
        select * into v_existing from public.legacy_records
            where kind = v_r ->> 'kind' and legacy_id = v_r ->> 'legacy_id';
        if found then
            if v_existing.body = v_r -> 'body' then
                v_entries := array_append(v_entries, jsonb_build_object(
                    'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                    'authority_id', v_existing.authority_id, 'status', 'already', 'reason', '',
                    'source', coalesce(v_r ->> 'source', '')));
            else
                v_entries := array_append(v_entries, jsonb_build_object(
                    'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                    'authority_id', null, 'status', 'quarantined',
                    'reason', 'differs from the imported record ' || v_existing.authority_id,
                    'source', coalesce(v_r ->> 'source', '')));
            end if;
            continue;
        end if;
        v_missing := null;
        for v_ref in select e from jsonb_array_elements(coalesce(v_r -> 'refs', '[]'))
            with ordinality t(e, n) order by n
        loop
            if not exists (select 1 from public.legacy_records
                    where kind = v_ref ->> 0 and legacy_id = v_ref ->> 1) then
                v_missing := v_ref;
                exit;
            end if;
        end loop;
        if v_missing is not null then
            v_entries := array_append(v_entries, jsonb_build_object(
                'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                'authority_id', null, 'status', 'quarantined',
                'reason', 'unresolved reference ' || (v_missing ->> 0) || ' ' || (v_missing ->> 1),
                'source', coalesce(v_r ->> 'source', '')));
            continue;
        end if;
        v_counts := jsonb_set(v_counts, array[v_r ->> 'kind'],
            to_jsonb(coalesce((v_counts ->> (v_r ->> 'kind'))::bigint, 0) + 1));
        v_aid := (v_r ->> 'kind') || '-' || (v_counts ->> (v_r ->> 'kind'));
        insert into public.legacy_records (authority_id, kind, legacy_id, body, manifest)
            values (v_aid, v_r ->> 'kind', v_r ->> 'legacy_id', v_r -> 'body', v_id);
        v_entries := array_append(v_entries, jsonb_build_object(
            'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
            'authority_id', v_aid, 'status', 'imported', 'reason', '',
            'source', coalesce(v_r ->> 'source', '')));
    end loop;
    insert into public.legacy_manifests (id, digest, version, entries)
        values (v_id, p_terms ->> 'digest', p_terms ->> 'version', to_jsonb(v_entries));
    perform alm_private.bump();
    return alm_private.record(p_request_id, 'import_legacy', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

-- Bills with crate rows in the projection rows (as `20261128000000_mixed_bills`), computed in
-- one pass: each crate or mixed bill's listing items and line count, sale-cost revenue and book
-- fields are built once, keyed by draft, then merged into the Sales, sale-cost and book arrays
-- in a single walk of each (the earlier version rewrote the whole arrays once per crate bill).
create or replace function alm_private.crate_rows(p_out jsonb)
returns jsonb
language plpgsql
stable
set search_path = ''
as $$
declare
    v_rows jsonb := p_out -> 'rows';
    v_over jsonb;
begin
    with crated as (
        select s.id, s.draft_id, s.crate_row, s.crate_packs, s.checkout_loading,
            coalesce(s.crate_rows, jsonb_build_array(s.crate_row)) as crates
        from public.sales s
        where s.crate_row is not null or s.crate_rows is not null
    ), shaped as (
        select c.*, (select count(*) from public.sale_lines l where l.sale_id = c.id)
            - alm_private.crate_components(c.crates) as ordinary
        from crated c
    )
    select coalesce(jsonb_object_agg(x.draft_id, jsonb_build_object(
            'items', coalesce((select jsonb_agg(l.item order by l.position)
                    from public.sale_lines l where l.sale_id = x.id
                        and l.position < x.ordinary and l.item is not null), '[]')
                || coalesce((select jsonb_agg(coalesce(r ->> 'name', '') order by n)
                    from jsonb_array_elements(x.crates) with ordinality t(r, n)), '[]'),
            'lines', x.ordinary + jsonb_array_length(x.crates),
            'revenue', coalesce((select sum(l.amount_minor) from public.sale_lines l
                    where l.sale_id = x.id and l.position < x.ordinary), 0)
                + coalesce((select sum((r ->> 'rate')::bigint * (r ->> 'quantity')::bigint)
                    from jsonb_array_elements(x.crates) r), 0),
            'book', jsonb_build_object(
                'lines', coalesce((select jsonb_agg(jsonb_build_array(l.product_id,
                        l.quantity) order by l.position) from public.sale_lines l
                    where l.sale_id = x.id and l.position < x.ordinary), '[]'),
                'prices', coalesce((select jsonb_agg(l.unit_price_minor order by l.position)
                        from public.sale_lines l where l.sale_id = x.id
                            and l.position < x.ordinary), '[]')
                    || coalesce((select jsonb_agg((r ->> 'rate')::bigint order by n)
                        from jsonb_array_elements(x.crates) with ordinality t(r, n)), '[]'),
                'crate_rows', x.crates,
                'checkout', case when x.crate_row is not null then jsonb_build_object(
                    'lines', '[]'::jsonb, 'loading', x.checkout_loading) end,
                'crate_packs', x.crate_packs))), '{}')
        into v_over
        from shaped x;
    v_rows := jsonb_set(v_rows, '{sales}', coalesce((select jsonb_agg(case
            when v_over ? (e ->> 'draft_id') then e || jsonb_build_object(
                'items', v_over -> (e ->> 'draft_id') -> 'items',
                'lines', v_over -> (e ->> 'draft_id') -> 'lines')
            else e end order by n)
        from jsonb_array_elements(v_rows -> 'sales') with ordinality t(e, n)), '[]'));
    v_rows := jsonb_set(v_rows, '{owner,sale_costs}', coalesce((select jsonb_agg(case
            when v_over ? (e ->> 0) then jsonb_build_array(e -> 0,
                v_over -> (e ->> 0) -> 'revenue', e -> 2)
            else e end order by n)
        from jsonb_array_elements(v_rows #> '{owner,sale_costs}') with ordinality t(e, n)),
        '[]'));
    v_rows := jsonb_set(v_rows, '{owner,book,sales}', coalesce((select jsonb_agg(
            (e - 'crate_row') || case
                when v_over ? (e ->> 'draft') then v_over -> (e ->> 'draft') -> 'book'
                else jsonb_build_object('crate_rows', '[]'::jsonb) end
            order by n)
        from jsonb_array_elements(v_rows #> '{owner,book,sales}') with ordinality t(e, n)),
        '[]'));
    return jsonb_set(p_out, '{rows}', v_rows);
end;
$$;


create or replace function alm_private.pull_projection_body(p_checkpoint bigint)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
declare
    v_role text := alm_private.app_role();
    v_rev bigint := (select revision from public.authority_state where id = 1);
    v_since bigint := coalesce(p_checkpoint, -1);
    v_rows jsonb;
    v_balances jsonb;
begin
    if v_role not in ('owner', 'manager') then
        raise exception 'denied' using errcode = 'ALM06';
    end if;
    if p_checkpoint is not null and p_checkpoint > v_rev then
        raise exception 'invalid_terms: checkpoint ahead' using errcode = 'ALM05';
    end if;
    if p_checkpoint = v_rev then
        return jsonb_build_object('role', v_role, 'checkpoint', v_rev,
            'as_of_ms', (extract(epoch from clock_timestamp()) * 1000)::bigint,
            'full', false, 'rows', null, 'removed', '[]'::jsonb);
    end if;
    -- The catalog a draft is made from: directory products at their provisional rate.
    v_rows := jsonb_build_object('catalog', coalesce((select jsonb_agg(jsonb_build_object(
            'id', id, 'name', item, 'rate', provisional_minor) order by id)
        from public.products where provisional_minor is not null and rev > v_since), '[]'));
    if v_role = 'owner' then
        -- Each customer's balance once, not once per bill.
        v_balances := coalesce((select jsonb_object_agg(c.id, alm_private.customer_balance(c.id))
            from public.customers c), '{}');
        v_rows := v_rows || jsonb_build_object(
            'sales_unsupported', null,
            'products', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'item', item,
                    'brand', brand, 'sell', sell_minor, 'reorder_level', reorder_level)
                    order by ord)
                from public.products where rev > v_since), '[]'),
            'stock', coalesce((select jsonb_agg(jsonb_build_object('product', id, 'item', item,
                    'brand', brand, 'on_hand', stock, 'reorder_level', reorder_level)
                    order by stock, ord)
                from public.products where rev > v_since), '[]'),
            'customers', coalesce((select jsonb_agg(jsonb_build_object('id', c.id, 'name', c.name,
                    'balance', (v_balances ->> c.id)::bigint) order by c.name)
                from public.customers c), '[]'),
            'sales', coalesce((select jsonb_agg(jsonb_build_object('draft_id', draft_id,
                    'invoice', invoice_no, 'date', sale_date, 'customer', customer_name,
                    'items', (select coalesce(jsonb_agg(item order by position), '[]')
                        from public.sale_lines l where l.sale_id = s.id),
                    'lines', (select count(*) from public.sale_lines l where l.sale_id = s.id),
                    'packets', (select coalesce(sum(quantity), 0) from public.sale_lines l
                        where l.sale_id = s.id),
                    'total', total_minor, 'paid', paid_minor, 'online', false,
                    'cancelled', cancelled,
                    'balance', case when cancelled then 0
                        when customer_id is null then total_minor - paid_minor
                        else coalesce((v_balances ->> customer_id)::bigint,
                            alm_private.customer_balance(customer_id)) end)
                    order by id desc)
                from public.sales s where rev > v_since), '[]'),
            'receipts', coalesce((select jsonb_agg(jsonb_build_object('slot', slot,
                    'date', receipt_date, 'customer', name, 'amount', amount_minor,
                    'online', online) order by slot)
                from (select r.*, c.name, row_number() over (order by r.id) - 1 as slot
                    from public.receipts r join public.customers c on c.id = r.customer_id) r
                where rev > v_since), '[]'),
            'cash_days', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'date', day,
                    'cash', cash_minor, 'paytm', paytm_minor, 'phonepe', phonepe_minor,
                    'goods', goods_minor, 'misc', misc_minor, 'salary', salary_minor,
                    'debt', debt_minor) order by day desc, id desc)
                from public.cash_days where rev > v_since), '[]'),
            'suppliers', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'name', name)
                    order by id) from public.suppliers where rev > v_since), '[]'),
            'crates', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'name', name,
                    'price', price_minor, 'components',
                    (select count(*) from public.crate_components c where c.crate_id = t.id))
                    order by id) from public.crate_templates t where rev > v_since), '[]'),
            'godown', coalesce((select jsonb_agg(jsonb_build_object('item', item, 'cases', cases,
                    'note', coalesce(note, '')) order by key)
                from public.godown_rows where rev > v_since), '[]'),
            'gst_bills', coalesce((select jsonb_agg(jsonb_build_object('id',
                    (body ->> 'id')::bigint, 'number', number, 'date', body ->> 'date',
                    'customer', body ->> 'customer', 'cancelled', cancelled) order by number)
                from public.gst_bills where body is not null and rev > v_since), '[]'),
            'owner', jsonb_build_object(
                'buy_rates', coalesce((select jsonb_agg(jsonb_build_array(id, buy_minor)
                        order by ord) from public.products where rev > v_since), '[]'),
                'supplier_prices', coalesce((select jsonb_agg(jsonb_build_array(q.product_id,
                        q.supplier_id, q.rate_minor) order by p.ord, q.supplier_id)
                    from public.supplier_prices q join public.products p on p.id = q.product_id
                    where q.rev > v_since), '[]'),
                'purchase_orders', coalesce((select jsonb_agg(jsonb_build_object('id', o.id,
                        'supplier', o.supplier_name, 'date', o.order_date,
                        'status', case
                            when not exists (select 1 from public.purchase_lines l
                                where l.po_id = o.id and l.ordered - l.received - l.closed > 0)
                                then 'Completed'
                            when exists (select 1 from public.purchase_lines l
                                where l.po_id = o.id and (l.received > 0 or l.closed > 0))
                                then 'PartiallyReceived'
                            else 'Pending' end,
                        'lines', (select jsonb_agg(jsonb_build_object('item', item,
                                'product', product_id, 'ordered', ordered, 'received', received,
                                'remaining', ordered - received - closed, 'rate', rate_minor)
                                order by position)
                            from public.purchase_lines l where l.po_id = o.id)) order by o.id)
                    from public.purchase_orders o where rev > v_since), '[]'),
                'stock_value', (select case when bool_and(unit_cost_minor is not null)
                        then coalesce(sum(remaining * unit_cost_minor), 0) end
                    from public.product_lots where remaining > 0),
                'sale_costs', coalesce((select jsonb_agg(jsonb_build_array(s.draft_id,
                        (select sum(amount_minor) from public.sale_lines l where l.sale_id = s.id),
                        s.cost_minor) order by s.id desc)
                    from public.sales s where not s.cancelled and s.rev > v_since), '[]')));
    end if;
    return jsonb_build_object('role', v_role, 'checkpoint', v_rev,
        'as_of_ms', (extract(epoch from clock_timestamp()) * 1000)::bigint,
        'full', p_checkpoint is null, 'rows', v_rows,
        'removed', coalesce((select jsonb_agg(jsonb_build_object('kind', kind, 'key', key)
                order by rev) from public.projection_removals
            where rev > v_since and (v_role = 'owner' or kind = 'product')), '[]'));
end;
$$;

create or replace function alm_private.owner_book()
returns jsonb
language sql
stable
set search_path = ''
as $$
    -- Each lot's place among its product's lots, numbered once for every bill.
    with lot_index as materialized (
        select id, product_id, unit_cost_minor,
            row_number() over (partition by product_id order by id) - 1 as idx
        from public.product_lots)
    select jsonb_build_object(
        'settings', (select jsonb_build_object(
                'default_product', a.default_product, 'reorder_default', a.reorder_default,
                'loading', a.loading_minor, 'crate_rate', a.crate_rate_minor,
                'next_invoice', (select next_no from public.invoice_counters where series = 'sale'),
                'movement', a.movement,
                'ledger', coalesce((select sum(total_minor) from public.sales where not cancelled), 0),
                'received_ledger', coalesce((select sum(paid_minor) from public.sales
                    where not cancelled), 0),
                'next_cash', a.next_cash)
            from public.authority_state a where a.id = 1),
        'products', coalesce((select jsonb_agg(jsonb_build_object(
                'id', p.id,
                'details', jsonb_build_object('item', p.item, 'brand', p.brand,
                    'reorder_level', p.reorder_level, 'gst_rate', p.gst_rate, 'cess', p.cess,
                    'hsn', p.hsn),
                'buy', p.buy_minor, 'margin', p.sell_minor - p.buy_minor, 'sell', p.sell_minor,
                'stock', p.stock, 'seeded_opening', p.seeded_opening, 'fridge', p.fridge,
                'lots', coalesce((select jsonb_agg(jsonb_build_array(l.received_on,
                        l.remaining, l.unit_cost_minor) order by l.id)
                    from public.product_lots l where l.product_id = p.id), '[]'),
                'counts', coalesce((select jsonb_agg(jsonb_build_object('date', c.count_date,
                        'from', c.from_qty, 'to', c.to_qty, 'delta', c.to_qty - c.from_qty,
                        'reason', c.reason, 'note', c.note) order by c.id)
                    from public.stock_counts c where c.product_id = p.id), '[]'))
                order by p.ord)
            from public.products p), '[]'),
        'checkout_terms', coalesce((select jsonb_agg(jsonb_build_array(product_id, pack,
                piece_minor, case_minor) order by product_id)
            from public.checkout_terms), '[]'),
        'sales', coalesce((select jsonb_agg(jsonb_build_object(
                'draft', s.draft_id, 'invoice', s.invoice_no,
                'customer', case when s.customer_id is not null
                    then jsonb_build_object('kind', 'selected', 'id', s.customer_id,
                        'name', s.customer_name)
                    else jsonb_build_object('kind', 'typed', 'name', s.customer_name) end,
                'date', s.sale_date, 'loading', s.loading_minor,
                'lines', coalesce((select jsonb_agg(jsonb_build_array(l.product_id, l.quantity)
                        order by l.position) from public.sale_lines l where l.sale_id = s.id), '[]'),
                'prices', coalesce((select jsonb_agg(l.unit_price_minor order by l.position)
                        from public.sale_lines l where l.sale_id = s.id), '[]'),
                'crate_row', null, 'checkout', null, 'crate_packs', null,
                'payment', jsonb_build_object('kind', 'named', 'amount', s.paid_minor),
                'online', false,
                'quantity', (select coalesce(sum(quantity), 0) from public.sale_lines l
                    where l.sale_id = s.id),
                'total', s.total_minor, 'received', s.paid_minor,
                'due', s.total_minor - s.paid_minor, 'customer_balance', s.balance_after,
                'edits', s.edits, 'cancelled', s.cancelled, 'epoch', s.epoch,
                'creation_key', null,
                'items', coalesce((select jsonb_agg(jsonb_build_array(l.product_id, l.item)
                        order by l.position) from public.sale_lines l where l.sale_id = s.id), '[]'),
                'cost', s.cost_minor,
                'line_units', coalesce((select jsonb_agg(jsonb_build_array(l.position,
                        l.frozen_unit) order by l.position)
                    from public.sale_lines l where l.sale_id = s.id and l.frozen), '[]'),
                'allocations', coalesce((select jsonb_agg(jsonb_build_array(a.position,
                        lot.product_id, lot.idx, a.quantity, a.quantity * lot.unit_cost_minor)
                        order by a.position, lot.product_id, lot.idx)
                    from public.sale_line_lots a
                    join lot_index lot on lot.id = a.lot_id
                    where a.sale_id = s.id), '[]'))
                order by s.id)
            from public.sales s), '[]'),
        'receipts', coalesce((select jsonb_agg(jsonb_build_object('customer', r.customer_id,
                'amount', r.amount_minor, 'date', r.receipt_date,
                'after_invoice', r.after_invoice, 'online', r.online,
                'selected_name', r.selected_name, 'creation_key', r.created_ms) order by r.id)
            from public.receipts r), '[]'),
        'customers', coalesce((select jsonb_agg(jsonb_build_object('id', c.id,
                'registered', c.registered,
                'balance', case when c.has_balance then alm_private.customer_balance(c.id) end,
                'has_opening', c.has_opening, 'opening', c.opening_minor,
                'opening_date', c.opening_date) order by c.id)
            from public.customers c
            where c.registered or c.has_balance or c.has_opening), '[]'),
        'purchasing', jsonb_build_object(
            'products', coalesce((select jsonb_agg(jsonb_build_array(id, purchase_item,
                    purchase_buy_minor) order by ord)
                from public.products where purchase_item is not null), '[]'),
            'suppliers', coalesce((select jsonb_agg(jsonb_build_array(id, name) order by id)
                from public.suppliers), '[]'),
            'price_suppliers', coalesce((select jsonb_agg(id order by id)
                from public.suppliers where price_column), '[]'),
            'costs', coalesce((select jsonb_agg(jsonb_build_array(id, purchase_avg_minor)
                    order by purchase_cost_seq)
                from public.products where purchase_avg_minor is not null), '[]'),
            'quotes', coalesce((select jsonb_agg(jsonb_build_array(p.id, (select jsonb_agg(
                        jsonb_build_array(s.name, q.rate_minor) order by q.seq)
                    from public.supplier_prices q join public.suppliers s on s.id = q.supplier_id
                    where q.product_id = p.id)) order by p.id)
                from public.products p
                where exists (select 1 from public.supplier_prices q where q.product_id = p.id)),
                '[]'),
            'orders', coalesce((select jsonb_agg(jsonb_build_object('id', o.id,
                    'supplier', o.supplier_id, 'supplier_name', o.supplier_name,
                    'date', o.order_date, 'note', o.note, 'cancelled', o.cancelled,
                    'lines', coalesce((select jsonb_agg(jsonb_build_object('item', l.item,
                            'product', l.product_id, 'ordered', l.ordered,
                            'received', l.received, 'closed', l.closed, 'rate', l.rate_minor,
                            'receipts', coalesce((select jsonb_agg(jsonb_build_array(
                                    r.receipt_date, r.quantity, r.rate_minor) order by r.seq)
                                from public.purchase_receipts r
                                where r.po_id = l.po_id and r.position = l.position), '[]'))
                            order by l.position)
                        from public.purchase_lines l where l.po_id = o.id), '[]'))
                    order by o.id)
                from public.purchase_orders o), '[]')),
        'crates', jsonb_build_object(
            'last_id', (select last_crate from public.authority_state where id = 1),
            'templates', coalesce((select jsonb_agg(jsonb_build_object('id', t.id,
                    'name', t.name, 'price', t.price_minor,
                    'components', coalesce((select jsonb_agg(jsonb_build_array(c.product_id,
                            c.quantity, c.piece) order by c.position)
                        from public.crate_components c where c.crate_id = t.id), '[]'))
                    order by t.id)
                from public.crate_templates t), '[]'),
            'piece_mrps', coalesce((select jsonb_agg(jsonb_build_array(product_id, mrp_minor)
                    order by product_id)
                from public.crate_piece_mrps), '[]')),
        'godown', coalesce((select jsonb_agg(jsonb_build_object('item', item, 'cases', cases,
                'note', note) order by key)
            from public.godown_rows), '[]'),
        'gst_bills', coalesce((select jsonb_agg(body || jsonb_build_object('number', number,
                'cancelled', cancelled) order by (body ->> 'id')::bigint)
            from public.gst_bills where body is not null), '[]'),
        'gst_deleted', coalesce((select jsonb_agg(substr(id, 5)::bigint
                order by substr(id, 5)::bigint)
            from public.gst_deleted where id ~ '^gst-[0-9]+$'), '[]'),
        'cash_days', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'date', day,
                'cash', cash_minor, 'paytm', paytm_minor, 'phonepe', phonepe_minor,
                'goods', goods_minor, 'misc', misc_minor, 'salary', salary_minor,
                'debt', debt_minor) order by id)
            from public.cash_days), '[]'));
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.import_legacy(text, jsonb) from public, anon;
grant execute on function public.import_legacy(text, jsonb) to authenticated, service_role;
