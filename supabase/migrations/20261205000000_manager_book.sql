-- Local development schema, continued: what a Manager posts, edits and cancels bills from
-- (DEC-034). A Manager's pull carries `manager`: the Owner book rows without any cost
-- (`alm_private.manager_book`, the core's `book::manager_json`: no buy rate, margin, FIFO lot,
-- stock count, sale cost or allocation, no supplier, quote, purchase rate, GST bill, activity
-- or legacy row, and the Cash Book row of the shop's today only) and `loading`, the shop's New
-- Bill loading charge per packet, which a Manager's drafts start with. `set_loading_default`
-- (Owner only) stores that charge. `manager.book.*` lists are tracked for incremental pulls
-- like `owner.book.*`.

alter table public.authority_state add column bill_loading_minor bigint not null default 0
    check (bill_loading_minor >= 0);

insert into alm_private.permissions (function_name, manager) values
    ('set_loading_default', false);

create function alm_private.set_loading_default_api(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
begin
    v_done := alm_private.replay(p_request_id, 'set_loading_default', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if jsonb_typeof(p_terms -> 'rate_minor') is distinct from 'number'
        or (p_terms ->> 'rate_minor')::numeric < 0
        or (p_terms ->> 'rate_minor')::numeric <> trunc((p_terms ->> 'rate_minor')::numeric)
        or (p_terms ->> 'rate_minor')::numeric > 9223372036854775807 then
        perform alm_private.invalid('loading charge');
    end if;
    perform alm_private.bump();
    update public.authority_state set bill_loading_minor = (p_terms ->> 'rate_minor')::bigint
        where id = 1;
    return alm_private.record(p_request_id, 'set_loading_default', p_terms, '{"done": true}');
end;
$$;

create function public.set_loading_default(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$ begin perform alm_private.require_permission('set_loading_default'); perform alm_private.manager_limits('set_loading_default', p_request_id, p_terms); return alm_private.set_loading_default_api(p_request_id, p_terms); end; $$;

revoke all on function alm_private.set_loading_default_api(text, jsonb)
    from public, anon, authenticated;
revoke all on function public.set_loading_default(text, jsonb) from public, anon;
grant execute on function public.set_loading_default(text, jsonb) to authenticated, service_role;

-- The Owner book without costs (see the header).
create function alm_private.manager_book(p_book jsonb, p_today date)
returns jsonb
language sql
stable
set search_path = ''
as $$
    select jsonb_build_object(
        'settings', p_book -> 'settings',
        'products', coalesce((select jsonb_agg(p - 'buy' - 'margin' - 'lots' - 'counts'
                order by n)
            from jsonb_array_elements(p_book -> 'products') with ordinality t(p, n)), '[]'),
        'checkout_terms', p_book -> 'checkout_terms',
        'sales', coalesce((select jsonb_agg(s - 'cost' - 'line_units' - 'allocations' order by n)
            from jsonb_array_elements(p_book -> 'sales') with ordinality t(s, n)), '[]'),
        'receipts', p_book -> 'receipts',
        'customers', p_book -> 'customers',
        'purchasing', jsonb_build_object('orders', coalesce((select jsonb_agg(
                o || jsonb_build_object('lines', coalesce((select jsonb_agg(
                        (l - 'rate' - 'receipts') || jsonb_build_object('receipts',
                            coalesce((select jsonb_agg(jsonb_build_array(r -> 0, r -> 1)
                                    order by rn)
                                from jsonb_array_elements(l -> 'receipts')
                                    with ordinality v(r, rn)), '[]'))
                        order by ln)
                    from jsonb_array_elements(o -> 'lines') with ordinality u(l, ln)), '[]'))
                order by n)
            from jsonb_array_elements(p_book #> '{purchasing,orders}') with ordinality t(o, n)),
            '[]')),
        'crates', p_book -> 'crates',
        'godown', p_book -> 'godown',
        'cash_days', coalesce((select jsonb_agg(c order by n)
            from jsonb_array_elements(p_book -> 'cash_days') with ordinality t(c, n)
            where c ->> 'date' = p_today::text), '[]'));
$$;

create or replace function alm_private.pull_projection_api(p_checkpoint bigint)
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
    v_out jsonb := alm_private.pull_projection_body(p_checkpoint);
    v_owner jsonb;
begin
    if v_out ->> 'role' = 'owner' and jsonb_typeof(v_out -> 'rows') = 'object' then
        v_out := jsonb_set(v_out, '{rows,owner,book}', alm_private.owner_book());
        v_out := alm_private.crate_rows(v_out);
        v_out := alm_private.customer_contacts(v_out);
        v_out := alm_private.activity_rows(v_out);
        v_out := alm_private.legacy_rows(v_out);
    elsif v_out ->> 'role' = 'manager' and jsonb_typeof(v_out -> 'rows') = 'object' then
        v_owner := alm_private.crate_rows(jsonb_build_object('rows', jsonb_build_object(
            'sales', '[]'::jsonb, 'owner', jsonb_build_object('sale_costs', '[]'::jsonb,
                'book', alm_private.owner_book()))));
        v_out := jsonb_set(v_out, '{rows,manager}', jsonb_build_object(
            'book', alm_private.manager_book(v_owner #> '{rows,owner,book}',
                alm_private.shop_today()),
            'loading', (select bill_loading_minor from public.authority_state where id = 1)));
    end if;
    return v_out;
end;
$$;

revoke all on function alm_private.manager_book(jsonb, date) from public, anon, authenticated;
revoke all on function alm_private.pull_projection_api(bigint) from public, anon, authenticated;

create or replace function alm_private.row_lists(p_rows jsonb)
returns table (path text, items jsonb)
language sql
immutable
set search_path = ''
as $$
    select e.key, e.value from pg_catalog.jsonb_each(p_rows) e
        where pg_catalog.jsonb_typeof(e.value) = 'array'
    union all
    select 'owner.' || e.key, e.value
        from pg_catalog.jsonb_each(case when pg_catalog.jsonb_typeof(p_rows -> 'owner') = 'object'
            then p_rows -> 'owner' else '{}'::jsonb end) e
        where pg_catalog.jsonb_typeof(e.value) = 'array'
    union all
    select 'owner.book.' || e.key, e.value
        from pg_catalog.jsonb_each(
            case when pg_catalog.jsonb_typeof(p_rows -> 'owner' -> 'book') = 'object'
            then p_rows -> 'owner' -> 'book' else '{}'::jsonb end) e
        where pg_catalog.jsonb_typeof(e.value) = 'array'
    union all
    select 'manager.book.' || e.key, e.value
        from pg_catalog.jsonb_each(
            case when pg_catalog.jsonb_typeof(p_rows -> 'manager' -> 'book') = 'object'
            then p_rows -> 'manager' -> 'book' else '{}'::jsonb end) e
        where pg_catalog.jsonb_typeof(e.value) = 'array';
$$;
