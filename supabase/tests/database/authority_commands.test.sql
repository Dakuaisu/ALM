-- Every SalesAuthority function: Owner allowed, Manager allowed or denied as the DEC-034 matrix
-- says (an allowed Manager call runs in a rolled-back probe), no-role callers denied, stale and
-- invalid terms rejected with no partial effect, identical retry returning the original
-- outcome (also through request_status), conflicting reuse rejected; the projection pull is
-- role-filtered and incremental; client roles reach functions only. Calls run as the
-- `authenticated` role with the role claim in request.jwt.claims (auth.jwt()).
begin;
create extension if not exists pgtap with schema extensions;
select * from no_plan();

insert into public.products (id, item, sell_minor, buy_minor, stock, provisional_minor) values
    (1, 'Synthetic Lime', 12050, 7000, 50, 11800), (2, 'Synthetic Soda', 6250, 3800, 40, 5900);
insert into public.product_lots (product_id, remaining, unit_cost_minor) values (1, 50, 7000), (2, 40, 3800);
insert into public.customers (id, name) values ('c1', 'Synthetic Customer');
insert into public.suppliers (id, name) values (0, 'Synthetic Wholesale');
insert into public.supplier_prices (product_id, supplier_id, rate_minor) values (1, 0, 6900);

-- Runs `p_sql` as `authenticated` with `p_role` (or no role) in the JWT claims.
create function pg_temp.call(p_role text, p_sql text)
returns jsonb
language plpgsql
as $$
declare
    v jsonb;
begin
    perform set_config('request.jwt.claims', case when p_role is null then ''
        else jsonb_build_object('sub', 'tap-' || p_role,
            'app_metadata', jsonb_build_object('role', p_role))::text end, true);
    execute 'set local role authenticated';
    execute p_sql into v;
    execute 'reset role';
    return v;
end;
$$;

-- A fingerprint of every public table and the checkpoint.
create function pg_temp.digest()
returns text
language plpgsql
as $$
declare
    t text;
    v text;
    acc text := '';
begin
    for t in select tablename from pg_tables where schemaname = 'public' order by tablename loop
        execute format('select coalesce(string_agg(x::text, %L order by x::text), %L) from public.%I x',
            ',', '-', t) into v;
        acc := acc || t || ':' || v || '|';
    end loop;
    return md5(acc);
end;
$$;

-- Runs `p_sql` as a Manager and rolls it back: 'committed' when it ran, else its SQLSTATE.
create function pg_temp.manager_probe(p_sql text)
returns text
language plpgsql
as $$
begin
    begin
        perform pg_temp.call('manager', p_sql);
        raise exception 'probe' using errcode = 'ALM99';
    exception when others then
        return case when sqlstate = 'ALM99' then 'committed' else sqlstate end;
    end;
end;
$$;

create function pg_temp.fn_sql(p_fn text, p_request text, p_terms jsonb)
returns text
language sql
as $$ select format('select public.%s(%L, %L::jsonb)', p_fn, p_request, p_terms) $$;

-- The common battery for one command function.
create function pg_temp.battery(
    p_tag text, p_fn text, p_good jsonb, p_other jsonb,
    p_stale jsonb, p_invalid jsonb, p_invalid_code text, p_manager boolean default false)
returns setof text
language plpgsql
as $$
declare
    v_request text := 'tap-' || p_tag;
    v_good text := pg_temp.fn_sql(p_fn, v_request, p_good);
    v_before text := pg_temp.digest();
    v_first jsonb;
begin
    if p_manager then
        return next is(pg_temp.manager_probe(v_good), 'committed',
            p_tag || ': a Manager may run it (DEC-034)');
    else
        return next throws_ok(format('select pg_temp.call(%L, %L)', 'manager', v_good), 'ALM06',
            null, p_tag || ': a Manager is denied');
    end if;
    return next throws_ok(format('select pg_temp.call(null, %L)', v_good), 'ALM06', null,
        p_tag || ': a caller without a role claim is denied');
    return next is(pg_temp.digest(), v_before, p_tag || ': denials leave no effect');
    if p_stale is not null then
        return next throws_ok(format('select pg_temp.call(%L, %L)', 'owner',
            pg_temp.fn_sql(p_fn, v_request || '-stale', p_stale)), 'ALM03', null,
            p_tag || ': stale reviewed terms are rejected');
        return next is(pg_temp.digest(), v_before, p_tag || ': the stale rejection leaves no effect');
    end if;
    if p_invalid is not null then
        return next throws_ok(format('select pg_temp.call(%L, %L)', 'owner',
            pg_temp.fn_sql(p_fn, v_request || '-invalid', p_invalid)), p_invalid_code, null,
            p_tag || ': invalid terms are rejected (' || p_invalid_code || ')');
        return next is(pg_temp.digest(), v_before, p_tag || ': the rejection leaves no partial effect');
    end if;
    v_first := pg_temp.call('owner', v_good);
    return next isnt(pg_temp.digest(), v_before, p_tag || ': the Owner call commits');
    v_before := pg_temp.digest();
    return next is(pg_temp.call('owner', v_good), v_first,
        p_tag || ': an identical retry returns the original outcome');
    return next is(pg_temp.digest(), v_before, p_tag || ': the retry has no second effect');
    return next is(pg_temp.call('owner', format('select public.request_status(%L)', v_request)) -> 'outcome',
        v_first, p_tag || ': request_status returns the committed outcome');
    return next throws_ok(format('select pg_temp.call(%L, %L)', 'manager',
        format('select public.request_status(%L)', v_request)), 'ALM06', null,
        p_tag || ': a Manager cannot read the Owner''s outcome');
    return next throws_ok(format('select pg_temp.call(%L, %L)', 'owner',
        pg_temp.fn_sql(p_fn, v_request, p_other)), 'ALM01', null,
        p_tag || ': reusing the request ID with other terms is a conflict');
    return next is(pg_temp.digest(), v_before, p_tag || ': the conflict leaves no effect');
end;
$$;

select * from pg_temp.battery('post-sale', 'post_sale',
    '{"draft_id": "p1", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 12050}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30", "paid_minor": 2000}',
    '{"draft_id": "p1", "lines": [{"product_id": 1, "quantity": 3, "unit_price_minor": 12050}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30", "paid_minor": 2000}',
    '{"draft_id": "p2", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 12051}]}',
    '{"draft_id": "p3", "lines": [{"product_id": 1, "quantity": 999, "unit_price_minor": 12050}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}',
    'ALM04', true);
select is((select stock from public.products where id = 1), 48, 'the sale took 2 from stock');
select is((select cost_minor from public.sale_lines), 14000::bigint, 'the sale line keeps its FIFO cost');

select * from pg_temp.battery('cancel-sale', 'set_sale_cancelled',
    '{"draft_id": "p1", "cancel": true}', '{"draft_id": "p1", "cancel": false}', null,
    '{"draft_id": "zz", "cancel": true}', 'ALM07', true);
select is((select stock from public.products where id = 1), 50, 'cancelling returned the stock');
select * from pg_temp.battery('restore-sale', 'set_sale_cancelled',
    '{"draft_id": "p1", "cancel": false}', '{"draft_id": "p1", "cancel": true}', null,
    '{"draft_id": "zz", "cancel": false}', 'ALM07');
select * from pg_temp.battery('edit-sale', 'edit_sale',
    '{"draft_id": "p1", "lines": [{"quantity": 3, "rate_minor": 12050}]}',
    '{"draft_id": "p1", "lines": [{"quantity": 4, "rate_minor": 12050}]}', null,
    '{"draft_id": "p1", "lines": [{"quantity": 3, "rate_minor": 12050}, {"quantity": 1, "rate_minor": 1}]}',
    'ALM05', true);
select is((select total_minor from public.sales where draft_id = 'p1'), 36150::bigint, 'the edit re-totals the bill');
select * from pg_temp.battery('receipt', 'record_receipt',
    '{"customer_id": "c1", "date": "2025-04-30", "amount_minor": 1000, "online": true}',
    '{"customer_id": "c1", "date": "2025-04-30", "amount_minor": 2000, "online": true}', null,
    '{"customer_id": "nobody", "date": "2025-04-30", "amount_minor": 1000, "online": true}', 'ALM07', true);
select * from pg_temp.battery('cash-day', 'save_cash_day',
    '{"id": null, "date": "2025-04-30", "amounts": {"cash": 2500, "paytm": 0, "phonepe": 0, "goods": 0, "misc": 0, "salary": 0, "debt": 0}}',
    '{"id": null, "date": "2025-04-30", "amounts": {"cash": 2600, "paytm": 0, "phonepe": 0, "goods": 0, "misc": 0, "salary": 0, "debt": 0}}',
    null,
    '{"id": 999, "date": "2025-04-30", "amounts": {"cash": 2500, "paytm": 0, "phonepe": 0, "goods": 0, "misc": 0, "salary": 0, "debt": 0}}',
    'ALM07');
select * from pg_temp.battery('cash-delete', 'save_cash_day',
    jsonb_build_object('id', (select max(id) from public.cash_days), 'delete', true),
    jsonb_build_object('id', (select max(id) from public.cash_days) + 1, 'delete', true), null,
    '{"id": 999, "delete": true}', 'ALM07');
select * from pg_temp.battery('product-new', 'save_product',
    '{"id": 777, "new": true, "item": "Tap Orange", "brand": "Tap", "buy_minor": 3000, "sell_minor": 4500, "reorder": 6, "reviewed": null}',
    '{"id": 777, "new": true, "item": "Tap Orange", "brand": "Tap", "buy_minor": 3000, "sell_minor": 4600, "reorder": 6, "reviewed": null}',
    null,
    '{"id": 777, "new": true, "item": " ", "brand": "Tap", "buy_minor": 3000, "sell_minor": 4500, "reorder": 6, "reviewed": null}',
    'ALM05');
select * from pg_temp.battery('product-edit', 'save_product',
    '{"id": 2, "new": false, "item": "Synthetic Soda", "brand": "", "buy_minor": 3800, "sell_minor": 6350, "reorder": null, "reviewed": {"buy_minor": 3800, "sell_minor": 6250}}',
    '{"id": 2, "new": false, "item": "Synthetic Soda", "brand": "", "buy_minor": 3800, "sell_minor": 6450, "reorder": null, "reviewed": {"buy_minor": 3800, "sell_minor": 6250}}',
    '{"id": 2, "new": false, "item": "Synthetic Soda", "brand": "", "buy_minor": 3800, "sell_minor": 6350, "reorder": null, "reviewed": {"buy_minor": 3800, "sell_minor": 6251}}',
    '{"id": 2, "new": false, "item": "Synthetic Lime", "brand": "", "buy_minor": 3800, "sell_minor": 6350, "reorder": null, "reviewed": null}',
    'ALM05');
select * from pg_temp.battery('product-delete', 'delete_product',
    '{"id": 777}', '{"id": 778}', null, '{"id": 1}', 'ALM08');
select * from pg_temp.battery('supplier', 'add_supplier',
    '{"name": "Tap Traders"}', '{"name": "Other Traders"}', null, '{"name": " "}', 'ALM05');
select * from pg_temp.battery('quotes', 'set_supplier_prices',
    '{"product_id": 1, "rates": [{"supplier_id": 0, "rate_minor": 2950}]}',
    '{"product_id": 1, "rates": [{"supplier_id": 0, "rate_minor": 2960}]}', null,
    '{"product_id": 1, "rates": [{"supplier_id": 0, "rate_minor": 2950}, {"supplier_id": 99, "rate_minor": 1}]}',
    'ALM07');
select * from pg_temp.battery('po', 'save_purchase_order',
    '{"supplier": {"id": null, "name": "Synthetic Wholesale"}, "date": "2025-04-30", "note": "", "lines": [{"item": "Synthetic Soda", "product_id": 2, "ordered": 10, "rate_minor": 3600}]}',
    '{"supplier": {"id": null, "name": "Synthetic Wholesale"}, "date": "2025-04-30", "note": "", "lines": [{"item": "Synthetic Soda", "product_id": 2, "ordered": 11, "rate_minor": 3600}]}',
    null,
    '{"supplier": null, "date": "2025-04-30", "note": "", "lines": [{"item": "Synthetic Soda", "product_id": 2, "ordered": 10, "rate_minor": 3600}]}',
    'ALM05');
select * from pg_temp.battery('receive', 'receive_purchase_order',
    '{"po": 0, "date": "2025-05-01", "today": "2025-05-01", "lines": [{"quantity": 4, "rate_minor": 3500}], "remaining": [10]}',
    '{"po": 0, "date": "2025-05-01", "today": "2025-05-01", "lines": [{"quantity": 5, "rate_minor": 3500}], "remaining": [10]}',
    '{"po": 0, "date": "2025-05-01", "today": "2025-05-01", "lines": [{"quantity": 4, "rate_minor": 3500}], "remaining": [9]}',
    '{"po": 0, "date": "2025-05-01", "today": "2025-05-01", "lines": [{"quantity": 0, "rate_minor": 0}], "remaining": [10]}',
    'ALM05');
select is((select stock from public.products where id = 2), 44, 'the partial receipt added 4 to stock');
select * from pg_temp.battery('close', 'close_purchase_order',
    '{"po": 0, "remaining": [6]}', '{"po": 0, "remaining": [5]}', '{"po": 0, "remaining": [10]}',
    '{"po": 99, "remaining": [6]}', 'ALM07');
select is((select ordered - received - closed from public.purchase_lines where po_id = 0), 0,
    'close remainder leaves nothing to receive');
select * from pg_temp.battery('crate', 'save_crate',
    '{"id": null, "name": "Tap Crate", "price_minor": 40000, "components": [{"product_id": 1, "quantity": 2, "piece": false}]}',
    '{"id": null, "name": "Tap Crate", "price_minor": 41000, "components": [{"product_id": 1, "quantity": 2, "piece": false}]}',
    null, '{"id": null, "name": "Tap Crate", "price_minor": 40000, "components": []}', 'ALM05', true);
select * from pg_temp.battery('crate-delete', 'delete_crate',
    '{"id": 1}', '{"id": 2}', null, '{"id": 99}', 'ALM07');
select * from pg_temp.battery('opening', 'save_opening',
    '{"customer_id": "c1", "amount_minor": 15000, "date": "2025-04-01"}',
    '{"customer_id": "c1", "amount_minor": 16000, "date": "2025-04-01"}', null,
    '{"customer_id": "nobody", "amount_minor": 15000, "date": null}', 'ALM07');
select * from pg_temp.battery('godown', 'save_godown',
    '{"item": "Tap Soda", "cases": "9", "note": "back room"}',
    '{"item": "Tap Soda", "cases": "10", "note": "back room"}', null,
    '{"item": "Tap Soda", "cases": "1.5", "note": ""}', 'ALM05', true);
select * from pg_temp.battery('count', 'count_stock',
    '{"product_id": 2, "counted": 41, "fridge": 2, "date": "2025-05-02", "reason": "damage", "note": "tap", "on_hand": 44}',
    '{"product_id": 2, "counted": 40, "fridge": 2, "date": "2025-05-02", "reason": "damage", "note": "tap", "on_hand": 44}',
    '{"product_id": 2, "counted": 41, "fridge": 2, "date": "2025-05-02", "reason": "damage", "note": "tap", "on_hand": 45}',
    '{"product_id": 99, "counted": 1, "fridge": null, "date": "2025-05-02", "reason": null, "note": null, "on_hand": 1}',
    'ALM07', true);

-- GST numbers: allocated by the authority, Owner only, idempotent per bill.
select throws_ok($$ select pg_temp.call('manager', 'select to_jsonb(public.allocate_gst_bill_number(''gst-50''))') $$,
    'ALM06', null, 'a Manager cannot allocate a GST number');
select is(pg_temp.call('owner', 'select to_jsonb(public.allocate_gst_bill_number(''gst-50''))'), '1'::jsonb,
    'the Owner allocates number 1');
select is(pg_temp.call('owner', 'select to_jsonb(public.allocate_gst_bill_number(''gst-50''))'), '1'::jsonb,
    'allocation is idempotent per bill');
select * from pg_temp.battery('gst-save', 'save_gst_bill',
    '{"bill": {"id": 50, "number": 1, "customer": "Tap Buyer", "date": "2025-04-30", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}}',
    '{"bill": {"id": 50, "number": 1, "customer": "Other Buyer", "date": "2025-04-30", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}}',
    '{"bill": {"id": 50, "number": 2, "customer": "Tap Buyer", "date": "2025-04-30", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}}',
    '{"bill": {"id": 50, "number": 1, "customer": "", "date": "2025-04-30", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}}',
    'ALM05');
select * from pg_temp.battery('gst-cancel', 'set_gst_bill_state',
    '{"id": 50, "delete": false}', '{"id": 50, "delete": true}', null, '{"id": 99, "delete": false}', 'ALM07');
select * from pg_temp.battery('gst-import', 'import_gst_bill',
    '{"bill": {"id": 60, "number": 60, "customer": "Moved Buyer", "date": "2025-04-20", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}, "deleted": null}',
    '{"bill": {"id": 60, "number": 61, "customer": "Moved Buyer", "date": "2025-04-20", "cancelled": false, "lines": [{"item": "Synthetic Water 1 L"}]}, "deleted": null}',
    null,
    '{"bill": {"id": 61, "number": 1, "customer": "Clash", "date": "2025-04-20", "cancelled": false, "lines": [{"item": "x"}]}, "deleted": null}',
    'ALM08');
select * from pg_temp.battery('gst-delete', 'set_gst_bill_state',
    '{"id": 60, "delete": true}', '{"id": 60, "delete": false}', null, '{"id": 99, "delete": true}', 'ALM07');
select is(pg_temp.call('owner', 'select to_jsonb(public.allocate_gst_bill_number(''gst-70''))'), '2'::jsonb,
    'deleting the highest-numbered bill frees its number (60 was deleted, 1 is the highest)');

-- Projection pull: role-filtered, incremental from a checkpoint.
select ok(pg_temp.call('owner', 'select public.pull_projection(null)') -> 'rows' ? 'owner',
    'the Owner pull carries owner values');
select is(pg_temp.call('owner', 'select public.pull_projection(null)') #> '{rows,owner,sale_costs,0}',
    '["p1", 36150, 21000]'::jsonb,
    'the Owner pull includes the sale revenue and the cost frozen at the edit (7000 a packet, 3 packets)');
select is(
    (select array_agg(k order by k) from jsonb_object_keys(
        pg_temp.call('manager', 'select public.pull_projection(null)') -> 'rows') k),
    array['catalog', 'customer_names'], 'a Manager pull holds only the catalog and customer names');
select is(
    (select array_agg(distinct k order by k) from jsonb_array_elements(
        pg_temp.call('manager', 'select public.pull_projection(null)') #> '{rows,customer_names}') e,
        jsonb_object_keys(e) k),
    array['id', 'name'], 'a Manager''s customer names carry no balance, mobile or address');
-- Structural: every key at every level, and every value except the pull's clock (`as_of_ms`,
-- milliseconds, which may hold any digits), carries no cost, buy-rate, FIFO, margin, profit,
-- quote or stock field, and none of the fixture's Owner-only amounts.
select ok(not exists (
    with recursive walk(key, value) as (
        select null::text, pg_temp.call('manager', 'select public.pull_projection(null)')
        union all
        select e.key, e.value from walk w cross join lateral (
            select o.key, o.value from jsonb_each(case when jsonb_typeof(w.value) = 'object'
                then w.value else '{}' end) o
            union all
            select w.key, a.value from jsonb_array_elements(case when jsonb_typeof(w.value) = 'array'
                then w.value else '[]' end) a) e
    )
    select 1 from walk where
        key ~* '(buy|cost|fifo|margin|profit|lot|stock_value|supplier|purchase|^stock$|on_hand|balance)'
        or (key is distinct from 'as_of_ms' and jsonb_typeof(value) not in ('object', 'array')
            and value #>> '{}' ~* '(buy|cost|fifo|margin|profit|lot|stock_value|supplier|purchase|on_hand|balance|7000|3800|3500|3600|2950)')),
    'a Manager response carries no cost, buy-rate, FIFO, margin, profit, quote or stock fields');
select is(jsonb_array_length(pg_temp.call('manager', 'select public.pull_projection(null)') #> '{rows,catalog}'), 2,
    'the Manager catalog lists the two products');
select throws_ok($$ select pg_temp.call(null, 'select public.pull_projection(null)') $$, 'ALM06', null,
    'a caller without a role claim cannot pull');
create temp table checkpoint_mark as
    select (pg_temp.call('owner', 'select public.pull_projection(null)') ->> 'checkpoint')::bigint as at;
select ok(pg_temp.call('owner', format('select public.pull_projection(%s)', (select at from checkpoint_mark))) -> 'rows'
    = 'null'::jsonb, 'nothing changed since the checkpoint: no rows');
select pg_temp.call('owner', pg_temp.fn_sql('save_godown', 'tap-incr', '{"item": "Tap Water", "cases": "3", "note": ""}'));
select is(pg_temp.call('owner', format('select public.pull_projection(%s)', (select at from checkpoint_mark))) #> '{rows,godown}',
    '[{"item": "Tap Water", "cases": 3, "note": ""}]'::jsonb, 'an incremental pull returns only the changed row');
select is(jsonb_array_length(pg_temp.call('owner', format('select public.pull_projection(%s)', (select at from checkpoint_mark))) #> '{rows,products}'),
    0, 'unchanged products are not resent');
select ok(pg_temp.call('owner', 'select public.pull_projection(0)') -> 'removed' @> '[{"kind": "product", "key": "777"}]',
    'removals since a checkpoint are listed');
select throws_ok($$ select pg_temp.call('owner', 'select public.pull_projection(999999)') $$, 'ALM05', null,
    'a checkpoint ahead of the authority is rejected');

-- Deny by default: functions only, row-level security on every table.
select is((select count(*) from pg_tables t join pg_class c on c.relname = t.tablename
        and c.relnamespace = 'public'::regnamespace
    where t.schemaname = 'public' and not c.relrowsecurity), 0::bigint,
    'row-level security is enabled on every public table');
select is((select count(*) from pg_class c where c.relnamespace = 'public'::regnamespace
    and c.relkind = 'r'
    and (has_table_privilege('authenticated', c.oid, 'select, insert, update, delete')
        or has_table_privilege('anon', c.oid, 'select, insert, update, delete'))),
    0::bigint, 'client roles have no table privileges');
select is((select count(*) from pg_proc p where p.pronamespace = 'public'::regnamespace
    and has_function_privilege('anon', p.oid, 'execute')), 0::bigint, 'anon executes no public function');
select is((select count(*) from pg_proc p where p.pronamespace = 'alm_private'::regnamespace
    and has_function_privilege('authenticated', p.oid, 'execute')), 0::bigint,
    'private helpers are not executable by clients');
select ok(has_function_privilege('authenticated', 'public.save_product(text, jsonb)', 'execute')
    and has_function_privilege('authenticated', 'public.pull_projection(bigint)', 'execute'),
    'authenticated reaches the command and pull functions');

select * from finish();
rollback;
