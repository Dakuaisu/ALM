-- Local development schema, continued: the DEC-034 Owner/Manager permission matrix (OPEN-004
-- decided). `alm_private.permissions` holds, per public function, whether a Manager may call
-- it; the rows are the core's `alm_core::permissions::sql_functions()` and a test compares the
-- two. Every public entry point's first statement becomes `require_permission(<its name>)`
-- (default deny: a function without a row is refused to both roles), then
-- `manager_limits`, which applies the narrower Manager rules of the functions a Manager shares
-- with the Owner:
--   set_sale_cancelled      cancel only (restore stays Owner-only);
--   save_cash_day           a new day for the shop's today (Asia/Kolkata) only, when today has
--                           no row yet; no edit, delete or other day;
--   receive_purchase_order  quantities only: no line may carry a rate (the order's rates apply);
--   request_status/lookup   only requests the same account sent.
-- A retry of the caller's own stored request skips the limits and replays as stored; another
-- account's request ID is refused to a Manager. Request rows now record the account (the JWT
-- `sub`) that sent them. The inner bodies on a Manager's path check membership only; the
-- entry point is the gate.

create table alm_private.permissions (
    function_name text primary key,
    manager boolean not null
);

insert into alm_private.permissions (function_name, manager) values
    ('add_supplier', false),
    ('allocate_gst_bill_number', false),
    ('close_purchase_order', false),
    ('count_stock', true),
    ('delete_crate', false),
    ('delete_product', false),
    ('edit_sale', true),
    ('import_gst_bill', false),
    ('import_legacy', false),
    ('post_sale', true),
    ('pull_changes', true),
    ('pull_projection', true),
    ('receive_purchase_order', true),
    ('record_receipt', true),
    ('record_restore', false),
    ('request_lookup', true),
    ('request_status', true),
    ('resolve_legacy_quarantine', false),
    ('save_cash_day', true),
    ('save_crate', true),
    ('save_godown', true),
    ('save_gst_bill', false),
    ('save_opening', false),
    ('save_product', false),
    ('save_purchase_order', false),
    ('sell_crate', true),
    ('set_gst_bill_state', false),
    ('set_reorder_default', false),
    ('set_sale_cancelled', true),
    ('set_supplier_prices', false);

create function alm_private.require_permission(p_function text)
returns void
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
    v_role text := alm_private.app_role();
begin
    if v_role = 'owner' and exists (select 1 from alm_private.permissions
            where function_name = p_function) then
        return;
    end if;
    if v_role = 'manager' and exists (select 1 from alm_private.permissions
            where function_name = p_function and manager) then
        return;
    end if;
    raise exception 'denied' using errcode = 'ALM06';
end;
$$;

-- The account a request comes from: the JWT subject ('' without one).
create function alm_private.caller_account()
returns text
language plpgsql
stable
set search_path = ''
as $$
begin
    return coalesce(auth.jwt() ->> 'sub', '');
exception when others then
    return '';
end;
$$;

-- The shop's calendar day (Asia/Kolkata), which a Manager's Cash Book close must be for.
create function alm_private.shop_today()
returns date
language sql
stable
set search_path = ''
as $$
    select (now() at time zone 'Asia/Kolkata')::date;
$$;

alter table public.sale_requests add column account text;
alter table public.command_requests add column account text;

create function alm_private.on_request_account()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    new.account := coalesce(new.account, alm_private.caller_account());
    return new;
end;
$$;

create trigger sale_requests_account before insert on public.sale_requests
    for each row execute function alm_private.on_request_account();
create trigger command_requests_account before insert on public.command_requests
    for each row execute function alm_private.on_request_account();

create function alm_private.manager_limits(p_function text, p_request_id text, p_terms jsonb)
returns void
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
    v_stored boolean;
    v_account text;
    v_day date;
begin
    if alm_private.app_role() <> 'manager' then
        return;
    end if;
    select true, account into v_stored, v_account from (
        select account from public.command_requests where request_id = p_request_id
        union all
        select account from public.sale_requests where request_id = p_request_id) r
        limit 1;
    if v_stored then
        if v_account is distinct from alm_private.caller_account() then
            raise exception 'denied' using errcode = 'ALM06';
        end if;
        return;
    end if;
    case p_function
    when 'set_sale_cancelled' then
        if (p_terms ->> 'cancel') is distinct from 'true' then
            raise exception 'denied' using errcode = 'ALM06';
        end if;
    when 'save_cash_day' then
        begin
            v_day := (p_terms ->> 'date')::date;
        exception when others then
            v_day := null;
        end;
        if p_terms ->> 'id' is not null or coalesce(p_terms ->> 'delete', 'false') <> 'false'
            or v_day is distinct from alm_private.shop_today()
            or exists (select 1 from public.cash_days where day = v_day) then
            raise exception 'denied' using errcode = 'ALM06';
        end if;
    when 'receive_purchase_order' then
        if jsonb_typeof(p_terms -> 'lines') = 'array' and exists (
                select 1 from jsonb_array_elements(p_terms -> 'lines') l
                where jsonb_typeof(l) <> 'object' or l ? 'rate_minor') then
            raise exception 'denied' using errcode = 'ALM06';
        end if;
    else
        null;
    end case;
end;
$$;

-- Every entry point: the matrix check, then the Manager limits, then the body as before.
do $$
declare
    f record;
    v_args text;
    v_limits text;
begin
    for f in
        select p.oid, p.proname, pg_catalog.pg_get_function_arguments(p.oid) as args,
            pg_catalog.pg_get_function_identity_arguments(p.oid) as ident,
            pg_catalog.pg_get_function_result(p.oid) as result, p.provolatile, p.proargnames
        from pg_catalog.pg_proc p
        where p.pronamespace = 'public'::regnamespace
            and pg_catalog.has_function_privilege('authenticated', p.oid, 'execute')
        order by p.proname
    loop
        select pg_catalog.string_agg(pg_catalog.quote_ident(a), ', ' order by i)
            into v_args from pg_catalog.unnest(f.proargnames) with ordinality u(a, i);
        v_limits := case
            when 'p_request_id' = any (f.proargnames) then format(
                'perform alm_private.manager_limits(%L, p_request_id, %s); ', f.proname,
                case when 'p_terms' = any (f.proargnames) then 'p_terms' else 'null' end)
            else '' end;
        execute format(
            'create or replace function public.%I(%s) returns %s language plpgsql security '
            'definer %s set search_path = '''' as $body$ begin '
            'perform alm_private.require_permission(%L); %s'
            'return alm_private.%I(%s); end; $body$',
            f.proname, f.args, f.result,
            case f.provolatile when 's' then 'stable' else 'volatile' end,
            f.proname, v_limits, f.proname || '_api', v_args);
    end loop;
end;
$$;

-- The bodies a Manager's commands run through check membership; the entry point checked the
-- matrix and the limits.
do $$
declare
    f record;
    v_def text;
begin
    for f in
        select p.oid from pg_catalog.pg_proc p
        where p.pronamespace = 'alm_private'::regnamespace
            and p.proname in ('post_sale_body', 'post_sale_lines_body', 'sell_crate_api',
                'edit_sale_api', 'edit_sale_body', 'set_sale_cancelled_api',
                'set_sale_cancelled_body', 'record_receipt_body', 'save_cash_day_api',
                'receive_purchase_order_body', 'save_crate_body', 'save_godown_api',
                'count_stock_api', 'request_status_api', 'request_lookup_api')
    loop
        v_def := pg_catalog.pg_get_functiondef(f.oid);
        if strpos(v_def, 'perform alm_private.require_owner();') = 0 then
            raise exception 'no owner check in %', f.oid::regprocedure;
        end if;
        execute replace(v_def, 'perform alm_private.require_owner();',
            'perform alm_private.require_member();');
    end loop;
end;
$$;

revoke all on function alm_private.require_permission(text) from public, anon, authenticated;
revoke all on function alm_private.manager_limits(text, text, jsonb)
    from public, anon, authenticated;
revoke all on function alm_private.caller_account() from public, anon, authenticated;
revoke all on function alm_private.shop_today() from public, anon, authenticated;
revoke all on function alm_private.on_request_account() from public, anon, authenticated;
