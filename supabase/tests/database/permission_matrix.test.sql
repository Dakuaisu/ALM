-- DEC-034 (migration 20261204000000): every function authenticated may call has a matrix row
-- and runs its check; walked for every function, a Manager is denied exactly the functions the
-- matrix keeps Owner-only, and no-role callers are denied all. The Manager limits of the shared
-- functions: cancel but not restore, today's Cash Book once, receiving without rates, and only
-- its own requests. Calls run as `authenticated` with the claims in request.jwt.claims.
begin;
create extension if not exists pgtap with schema extensions;
select no_plan();

-- The SQLSTATE of `p_sql` run as `p_role` (or no role), rolled back; 'ok' when it ran.
create function pg_temp.state(p_role text, p_sql text)
returns text
language plpgsql
as $$
begin
    begin
        perform set_config('request.jwt.claims', case when p_role is null then ''
            else jsonb_build_object('sub', 'tap-' || p_role,
                'app_metadata', jsonb_build_object('role', p_role))::text end, true);
        execute 'set local role authenticated';
        execute p_sql;
        raise exception 'probe' using errcode = 'ALM99';
    exception when others then
        return case when sqlstate = 'ALM99' then 'ok' else sqlstate end;
    end;
end;
$$;

-- Runs `p_sql` as `p_role` and keeps its effect.
create function pg_temp.run(p_role text, p_sql text)
returns void
language plpgsql
as $$
begin
    perform set_config('request.jwt.claims', jsonb_build_object('sub', 'tap-' || p_role,
        'app_metadata', jsonb_build_object('role', p_role))::text, true);
    execute 'set local role authenticated';
    execute p_sql;
    execute 'reset role';
end;
$$;

-- A Manager's full pull.
create function pg_temp.manager_pull()
returns jsonb
language plpgsql
as $$
declare
    v jsonb;
begin
    perform set_config('request.jwt.claims', '{"sub": "tap-manager", "app_metadata": {"role": "manager"}}', true);
    execute 'set local role authenticated';
    v := public.pull_projection(null);
    execute 'reset role';
    return v;
end;
$$;

-- A call of public function `p` with placeholder arguments of its types.
create function pg_temp.placeholder_call(p regprocedure)
returns text
language sql
as $$
    select format('select public.%I(%s)', pr.proname, coalesce(string_agg(
        case t.typname when 'jsonb' then quote_literal(case pr.proname
                -- Terms inside the Manager limits, so the walk sees the matrix row.
                when 'set_sale_cancelled' then '{"cancel": true}'
                when 'save_cash_day' then jsonb_build_object('date',
                    alm_private.shop_today()::text)::text
                else '{}' end) || '::jsonb'
            when 'text' then '''tap-matrix'''
            else 'null::' || format_type(t.oid, null) end, ', ' order by a.i), ''))
    from pg_proc pr
        left join lateral unnest(pr.proargtypes) with ordinality a(ty, i) on true
        left join pg_type t on t.oid = a.ty
    where pr.oid = p
    group by pr.proname
$$;

create temp table api as
    select p.oid::regprocedure as f, p.proname::text as name
    from pg_proc p
    where p.pronamespace = 'public'::regnamespace
        and has_function_privilege('authenticated', p.oid, 'execute');

select is_empty($$ select name from api where name not in
        (select function_name from alm_private.permissions) $$,
    'every function authenticated may call has a matrix row');
select is_empty($$ select function_name from alm_private.permissions where function_name not in
        (select name from api) $$,
    'every matrix row names a function authenticated may call');
select is_empty($$ select name from api a where pg_get_functiondef(a.f) not like
        '%perform alm_private.require_permission(''' || a.name || ''')%' $$,
    'every entry point checks its own matrix row');

select is(pg_temp.state('manager', pg_temp.placeholder_call(a.f)) = 'ALM06', not m.manager,
        'a Manager ' || case when m.manager then 'may call ' else 'is denied ' end || a.name)
    from api a join alm_private.permissions m on m.function_name = a.name order by a.name;
select is(pg_temp.state('owner', pg_temp.placeholder_call(a.f)) <> 'ALM06', true,
        'the Owner may call ' || a.name)
    from api a order by a.name;
select is(pg_temp.state(null, pg_temp.placeholder_call(a.f)), 'ALM06',
        'a caller without a role is denied ' || a.name)
    from api a order by a.name;

-- The Manager limits.
insert into public.products (id, item, sell_minor, buy_minor, stock) values
    (1, 'Synthetic Lime', 12050, 7000, 50);
insert into public.product_lots (product_id, remaining, unit_cost_minor) values (1, 50, 7000);
insert into public.customers (id, name) values ('c1', 'Synthetic Customer');
create temp table day_terms as select jsonb_build_object('id', null,
    'date', alm_private.shop_today()::text, 'amounts', jsonb_build_object('cash', 2500,
    'paytm', 0, 'phonepe', 0, 'goods', 0, 'misc', 0, 'salary', 0, 'debt', 0)) as t;

select pg_temp.run('manager', $$ select public.post_sale('mgr-sale', '{"draft_id": "m1", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 12050}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30", "paid_minor": 2000}') $$);
select is((select invoice_no from public.sales where draft_id = 'm1'), 1::bigint,
    'a Manager posts a bill (DEC-034)');
select is((select account from public.sale_requests where request_id = 'mgr-sale'), 'tap-manager',
    'the request records the account that sent it');
select is(pg_temp.state('manager', $$ select public.set_sale_cancelled('mgr-restore-1', '{"draft_id": "m1", "cancel": false}') $$),
    'ALM06', 'a Manager may not restore a bill');
select pg_temp.run('manager', $$ select public.set_sale_cancelled('mgr-cancel', '{"draft_id": "m1", "cancel": true}') $$);
select is((select cancelled from public.sales where draft_id = 'm1'), true, 'a Manager cancels a bill (DEC-034)');
select is(pg_temp.state('manager', $$ select public.set_sale_cancelled('mgr-restore', '{"draft_id": "m1", "cancel": false}') $$),
    'ALM06', 'nor restore the cancelled bill');
select pg_temp.run('owner', $$ select public.set_sale_cancelled('own-restore', '{"draft_id": "m1", "cancel": false}') $$);
select is((select cancelled from public.sales where draft_id = 'm1'), false, 'the Owner restores it');

select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day-old',
        (select t || '{"date": "2025-04-30"}' from day_terms))),
    'ALM06', 'a Manager may not close an earlier day');
select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day-next',
        (select t || jsonb_build_object('date', (alm_private.shop_today() + 1)::text) from day_terms))),
    'ALM06', 'nor a later day');
select pg_temp.run('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day',
    (select t from day_terms)));
select is((select cash_minor from public.cash_days where day = alm_private.shop_today()), 2500::bigint,
    'a Manager closes today (DEC-034)');
select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day',
        (select t from day_terms))), 'ok', 'and may retry its own close');
select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day-2',
        (select t from day_terms))), 'ALM06', 'but not close today a second time');
select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day-edit',
        (select t || jsonb_build_object('id', (select id from public.cash_days)) from day_terms))),
    'ALM06', 'nor edit the closed day');
select is(pg_temp.state('manager', format('select public.save_cash_day(%L, %L)', 'mgr-day-del',
        jsonb_build_object('id', (select id from public.cash_days), 'delete', true))),
    'ALM06', 'nor delete it');
select is(pg_temp.state('owner', format('select public.save_cash_day(%L, %L)', 'own-day-edit',
        (select t || jsonb_build_object('id', (select id from public.cash_days)) from day_terms))),
    'ok', 'the Owner edits it');

select is(pg_temp.state('manager', $$ select public.receive_purchase_order('mgr-recv', '{"po": 0, "date": null, "today": "2025-05-01", "lines": [{"quantity": 4, "rate_minor": 3500}], "remaining": [10]}') $$),
    'ALM06', 'a Manager may not send a buy rate when receiving');

select is(pg_temp.state('manager', $$ select public.request_status('mgr-sale') $$), 'ok',
    'a Manager checks the status of its own request');
select is(pg_temp.state('manager', $$ select public.request_status('own-restore') $$), 'ALM06',
    'but not of the Owner''s');
select is(pg_temp.state('manager', $$ select public.set_sale_cancelled('own-restore', '{"draft_id": "m1", "cancel": false}') $$),
    'ALM06', 'nor replay the Owner''s request');

-- Customers (DEC-034): a Manager adds one and changes its contact; the opening stays the Owner's.
select pg_temp.run('manager', $$ select public.save_customer('mgr-cust', '{"id": null, "name": " Synthetic Corner ", "mobile": "90000 00009", "addr1": "3 Synthetic Road", "addr2": ""}') $$);
select is((select name || '/' || mobile from public.customers where id = 'cust-1'),
    'Synthetic Corner/90000 00009', 'a Manager adds a customer, trimmed and numbered by the authority');
select is(pg_temp.state('manager', $$ select public.save_customer('mgr-twin', '{"id": null, "name": "synthetic corner", "mobile": "", "addr1": "", "addr2": ""}') $$),
    'ALM05', 'a second customer of the same name is refused');
select pg_temp.run('manager', $$ select public.save_customer('mgr-cust-edit', '{"id": "cust-1", "name": "Synthetic Corner", "mobile": "90000 00010", "addr1": "4 Synthetic Road", "addr2": "Kanpur"}') $$);
select is((select mobile || '/' || addr1 || '/' || addr2 from public.customers where id = 'cust-1'),
    '90000 00010/4 Synthetic Road/Kanpur', 'a Manager changes a customer''s contact');
select is(pg_temp.state('manager', $$ select public.save_opening('mgr-open', '{"customer_id": "cust-1", "amount_minor": 100, "date": null}') $$),
    'ALM06', 'the opening balance stays the Owner''s');
select is(pg_temp.state('manager', $$ select public.record_receipt('mgr-pay', '{"customer_id": "cust-1", "date": "2025-05-02", "amount_minor": 500, "online": true}') $$),
    'ok', 'a Manager records a payment');
select ok((select pull -> 'rows' -> 'customers' @> '[{"id": "cust-1", "mobile": "90000 00010", "balance": 0}]'
        from (select pg_temp.manager_pull() as pull) p),
    'a Manager''s pull carries customers with balances, mobiles and addresses');

-- Receiving (DEC-034): a Manager receives quantities only; the order's rate applies.
insert into public.products (id, item, sell_minor, buy_minor, stock) values
    (2, 'Synthetic Soda', 6250, 3800, 40);
insert into public.product_lots (product_id, remaining, unit_cost_minor) values (2, 40, 3800);
select pg_temp.run('owner', $$ select public.save_purchase_order('own-po', '{"supplier": {"id": null, "name": "Synthetic Depot"}, "date": "2025-04-30", "note": "", "lines": [{"item": "Synthetic Soda", "product_id": 2, "ordered": 10, "rate_minor": 3610}]}') $$);
select pg_temp.run('manager', format('select public.receive_purchase_order(%L, %L)', 'mgr-recv-qty',
    jsonb_build_object('po', (select max(id) from public.purchase_orders), 'date', '2025-04-30',
        'today', '2025-04-30', 'lines', '[{"quantity": 4}]'::jsonb, 'remaining', '[10]'::jsonb)));
select is((select stock from public.products where id = 2), 44, 'a Manager receives 4 packets');
select is((select unit_cost_minor from public.product_lots where product_id = 2 order by id desc limit 1),
    3610::bigint, 'at the order''s own rate');
select ok(not (pg_temp.manager_pull()::text ~ '3610|3800'),
    'the Manager''s pull carries neither the order''s rate nor the buy rate');
select is(pg_temp.state('manager', format('select public.close_purchase_order(%L, %L)', 'mgr-close',
        jsonb_build_object('po', (select max(id) from public.purchase_orders), 'remaining', '[6]'::jsonb))),
    'ALM06', 'closing a remainder stays the Owner''s');

-- The activity log names the account by its login email (DEC-034).
select set_config('request.jwt.claims', '{"sub": "tap-manager", "email": "manager@alm.test", "app_metadata": {"role": "manager"}}', true);
set local role authenticated;
select public.save_godown('mgr-godown-mail', '{"item": "Synthetic Ice", "cases": "2", "note": ""}');
reset role;
select is((select count(*) from public.activity where account = 'manager@alm.test'), 0::bigint,
    'a godown row logs nothing (as the HTML)');
select set_config('request.jwt.claims', '{"sub": "tap-manager", "email": "manager@alm.test", "app_metadata": {"role": "manager"}}', true);
set local role authenticated;
select public.record_receipt('mgr-pay-mail', '{"customer_id": "c1", "date": "2025-05-03", "amount_minor": 300, "online": false}');
reset role;
select is((select account from public.activity where kind = 'payment' order by id desc limit 1),
    'manager@alm.test', 'a Manager''s payment is logged under its login email');

select * from finish();
rollback;
