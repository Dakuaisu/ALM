-- Local development schema, continued: one transactional function per SalesAuthority command,
-- request identities with post_sale's idempotency and conflict rules, a JWT role gate and a
-- role-filtered projection pull. Synthetic slice only; not the deployed schema (OPEN-002) and
-- not an approved permission matrix (OPEN-004): the Owner may run every command, a Manager
-- none, and anything else is denied.
--
-- SQLSTATEs: ALM01 request_conflict, ALM02 draft_already_posted, ALM03 stale_review,
-- ALM04 insufficient_stock, ALM05 invalid_terms, ALM06 denied, ALM07 not_found,
-- ALM08 invalid_state.

create schema if not exists alm_private;
revoke all on schema alm_private from public;

-- The authority's checkpoint: every committed command advances it once.
create table public.authority_state (
    id integer primary key check (id = 1),
    revision bigint not null check (revision >= 0)
);
insert into public.authority_state values (1, 0);

-- Rows a device must drop on its next pull.
create table public.projection_removals (
    rev bigint not null,
    kind text not null,
    key text not null
);

alter table public.products
    add column brand text not null default '',
    add column buy_minor bigint not null default 0 check (buy_minor >= 0),
    add column reorder_level integer not null default 5 check (reorder_level >= 0),
    add column fridge integer not null default 0 check (fridge >= 0),
    add column rev bigint not null default 0;

-- FIFO layers; unit cost null when unknown. Owner-only.
create table public.product_lots (
    id bigint generated always as identity primary key,
    product_id bigint not null references public.products (id) on delete cascade,
    received_on date,
    remaining integer not null check (remaining >= 0),
    unit_cost_minor bigint check (unit_cost_minor >= 0)
);

create table public.customers (
    id text primary key check (length(id) between 1 and 64),
    name text not null,
    opening_minor bigint not null default 0,
    opening_date date,
    rev bigint not null default 0
);

alter table public.sales
    add column customer_id text,
    add column customer_name text not null default '',
    add column sale_date date,
    add column paid_minor bigint not null default 0 check (paid_minor >= 0),
    add column loading_minor bigint not null default 0 check (loading_minor >= 0),
    add column cancelled boolean not null default false,
    add column rev bigint not null default 0;
alter table public.sale_lines add column cost_minor bigint;

create table public.receipts (
    id bigint generated always as identity primary key,
    customer_id text not null references public.customers (id),
    receipt_date date not null,
    amount_minor bigint not null check (amount_minor <> 0),
    online boolean not null,
    rev bigint not null default 0
);

create table public.cash_days (
    id bigint generated always as identity primary key,
    day date not null,
    cash_minor bigint not null check (cash_minor >= 0),
    paytm_minor bigint not null check (paytm_minor >= 0),
    phonepe_minor bigint not null check (phonepe_minor >= 0),
    goods_minor bigint not null check (goods_minor >= 0),
    misc_minor bigint not null check (misc_minor >= 0),
    salary_minor bigint not null check (salary_minor >= 0),
    debt_minor bigint not null check (debt_minor >= 0),
    rev bigint not null default 0
);

create table public.suppliers (
    id bigint primary key check (id >= 0),
    name text not null,
    rev bigint not null default 0
);
create unique index suppliers_name on public.suppliers (lower(name));

-- Quoted buy rates: owner-only.
create table public.supplier_prices (
    product_id bigint not null references public.products (id) on delete cascade,
    supplier_id bigint not null references public.suppliers (id),
    rate_minor bigint not null check (rate_minor > 0),
    rev bigint not null default 0,
    primary key (product_id, supplier_id)
);

create table public.purchase_orders (
    id bigint primary key check (id >= 0),
    supplier_id bigint not null references public.suppliers (id),
    supplier_name text not null,
    order_date date not null,
    note text not null default '',
    rev bigint not null default 0
);

create table public.purchase_lines (
    po_id bigint not null references public.purchase_orders (id),
    position integer not null check (position >= 0),
    item text not null,
    product_id bigint,
    ordered integer not null check (ordered > 0),
    received integer not null default 0 check (received >= 0),
    closed integer not null default 0 check (closed >= 0),
    rate_minor bigint not null check (rate_minor >= 0),
    primary key (po_id, position),
    check (received + closed <= ordered)
);

create table public.crate_templates (
    id bigint primary key check (id >= 1),
    name text not null,
    price_minor bigint not null check (price_minor >= 0),
    rev bigint not null default 0
);

create table public.crate_components (
    crate_id bigint not null references public.crate_templates (id) on delete cascade,
    position integer not null,
    product_id bigint not null,
    quantity integer not null check (quantity > 0),
    piece boolean not null,
    primary key (crate_id, position)
);

create table public.godown_rows (
    key text primary key,
    item text not null,
    cases integer not null check (cases >= 0),
    note text,
    rev bigint not null default 0
);

create table public.stock_counts (
    id bigint generated always as identity primary key,
    product_id bigint not null references public.products (id) on delete cascade,
    count_date date not null,
    from_qty integer not null,
    to_qty integer not null,
    reason text,
    note text
);

alter table public.gst_bills
    add column rev bigint not null default 0;

create table public.gst_deleted (
    id text primary key
);

-- The idempotency record of every command other than post_sale (which keeps sale_requests).
create table public.command_requests (
    request_id text primary key check (length(request_id) between 1 and 64),
    command text not null,
    terms jsonb not null,
    outcome jsonb not null,
    created_at timestamptz not null default now()
);

do $$
declare
    t text;
begin
    foreach t in array array[
        'authority_state', 'projection_removals', 'product_lots', 'customers', 'receipts',
        'cash_days', 'suppliers', 'supplier_prices', 'purchase_orders', 'purchase_lines',
        'crate_templates', 'crate_components', 'godown_rows', 'stock_counts', 'gst_deleted',
        'command_requests'
    ] loop
        execute format('alter table public.%I enable row level security', t);
    end loop;
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Private helpers: not executable by client roles.

-- The caller's role from the JWT app_metadata (set by the server, not the user). Without a
-- JWT, or on plain Postgres without auth.jwt(), there is no role.
create function alm_private.app_role()
returns text
language plpgsql
stable
set search_path = ''
as $$
begin
    return coalesce(auth.jwt() -> 'app_metadata' ->> 'role', '');
exception
    when undefined_function or invalid_schema_name then
        return '';
end;
$$;

-- The current Rust gate: the Owner runs commands; a Manager and anyone else are denied.
create function alm_private.require_owner()
returns void
language plpgsql
stable
set search_path = ''
as $$
begin
    if alm_private.app_role() is distinct from 'owner' then
        raise exception 'denied' using errcode = 'ALM06';
    end if;
end;
$$;

create function alm_private.invalid(p_reason text)
returns void
language plpgsql
as $$
begin
    raise exception 'invalid_terms: %', p_reason using errcode = 'ALM05';
end;
$$;

-- Serializes a request ID; returns the stored outcome of an identical retry, raises
-- request_conflict for a reuse with another command or terms, and null for a new request.
create function alm_private.replay(p_request_id text, p_command text, p_terms jsonb)
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
    v public.command_requests%rowtype;
begin
    if p_request_id is null or length(p_request_id) not between 1 and 64 or p_terms is null then
        perform alm_private.invalid('request');
    end if;
    perform pg_advisory_xact_lock(hashtextextended('sale_request:' || p_request_id, 0));
    select * into v from public.command_requests where request_id = p_request_id;
    if found then
        if v.command = p_command and v.terms = p_terms then
            return v.outcome;
        end if;
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    if exists (select 1 from public.sale_requests where request_id = p_request_id) then
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    return null;
end;
$$;

-- Advances the checkpoint once per committed command and returns the new revision.
create function alm_private.bump()
returns bigint
language sql
set search_path = ''
as $$
    update public.authority_state set revision = revision + 1 where id = 1 returning revision;
$$;

create function alm_private.record(
    p_request_id text, p_command text, p_terms jsonb, p_outcome jsonb)
returns jsonb
language sql
set search_path = ''
as $$
    insert into public.command_requests (request_id, command, terms, outcome)
        values (p_request_id, p_command, p_terms, p_outcome)
        returning outcome;
$$;

create function alm_private.removed(p_rev bigint, p_kind text, p_key text)
returns void
language sql
set search_path = ''
as $$
    insert into public.projection_removals values (p_rev, p_kind, p_key);
$$;

-- Takes `p_quantity` packets from the oldest lots; returns their cost, or null when any taken
-- packet has no known cost. Stock itself is moved by the caller.
create function alm_private.take_lots(p_product bigint, p_quantity integer)
returns bigint
language plpgsql
set search_path = ''
as $$
declare
    v_lot record;
    v_left integer := p_quantity;
    v_take integer;
    v_cost bigint := 0;
    v_known boolean := true;
begin
    for v_lot in
        select id, remaining, unit_cost_minor from public.product_lots
        where product_id = p_product and remaining > 0
        order by received_on nulls first, id
        for update
    loop
        exit when v_left = 0;
        v_take := least(v_left, v_lot.remaining);
        update public.product_lots set remaining = remaining - v_take where id = v_lot.id;
        if v_lot.unit_cost_minor is null then
            v_known := false;
        else
            v_cost := v_cost + v_take * v_lot.unit_cost_minor;
        end if;
        v_left := v_left - v_take;
    end loop;
    if v_left > 0 then
        v_known := false;
    end if;
    return case when v_known then v_cost end;
end;
$$;

create function alm_private.text_date(p_value jsonb)
returns date
language plpgsql
immutable
as $$
begin
    return case when p_value is null or jsonb_typeof(p_value) = 'null' then null
        else (p_value #>> '{}')::date end;
exception when others then
    raise exception 'invalid_terms: date' using errcode = 'ALM05';
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Sales.

-- Replaces the first slice's post_sale: same terms, receipt and errors, plus the Owner gate,
-- optional customer_id/customer/date/paid_minor/loading_minor, FIFO cost and the checkpoint.
create or replace function public.post_sale(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_existing public.sale_requests%rowtype;
    v_draft text := p_terms ->> 'draft_id';
    v_line jsonb;
    v_product public.products%rowtype;
    v_position integer := 0;
    v_quantity integer;
    v_price bigint;
    v_total bigint := 0;
    v_units bigint := 0;
    v_loading bigint := coalesce((p_terms ->> 'loading_minor')::bigint, 0);
    v_paid bigint := coalesce((p_terms ->> 'paid_minor')::bigint, 0);
    v_invoice bigint;
    v_sale bigint;
    v_rev bigint;
    v_receipt jsonb;
begin
    perform alm_private.require_owner();
    if p_request_id is null or v_draft is null or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(p_terms -> 'lines') = 0 or v_loading < 0 or v_paid < 0 then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('sale_request:' || p_request_id, 0));
    select * into v_existing from public.sale_requests where request_id = p_request_id;
    if found then
        if v_existing.terms = p_terms then
            return v_existing.receipt;
        end if;
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    if exists (select 1 from public.command_requests where request_id = p_request_id) then
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('sale_draft:' || v_draft, 0));
    if exists (select 1 from public.sales where draft_id = v_draft) then
        raise exception 'draft_already_posted' using errcode = 'ALM02';
    end if;
    if p_terms ? 'customer_id' and not exists (
        select 1 from public.customers where id = p_terms ->> 'customer_id') then
        raise exception 'not_found: customer' using errcode = 'ALM07';
    end if;
    perform 1 from public.products
        where id in (select (l ->> 'product_id')::bigint from jsonb_array_elements(p_terms -> 'lines') l)
        order by id for update;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        select * into v_product from public.products where id = (v_line ->> 'product_id')::bigint;
        v_quantity := (v_line ->> 'quantity')::integer;
        v_price := (v_line ->> 'unit_price_minor')::bigint;
        if not found or v_quantity is null or v_quantity <= 0 or v_price is null then
            raise exception 'invalid_terms' using errcode = 'ALM05';
        end if;
        if v_price <> v_product.sell_minor then
            raise exception 'stale_review' using errcode = 'ALM03';
        end if;
        if v_product.stock < v_quantity then
            raise exception 'insufficient_stock' using errcode = 'ALM04';
        end if;
        v_total := v_total + v_price * v_quantity;
        v_units := v_units + v_quantity;
    end loop;
    v_total := v_total + v_loading * v_units;
    v_rev := alm_private.bump();
    update public.invoice_counters set next_no = next_no + 1
        where series = 'sale' returning next_no - 1 into v_invoice;
    insert into public.sales (invoice_no, draft_id, total_minor, customer_id, customer_name,
            sale_date, paid_minor, loading_minor, rev)
        values (v_invoice, v_draft, v_total, p_terms ->> 'customer_id',
            coalesce(p_terms ->> 'customer', ''), alm_private.text_date(p_terms -> 'date'),
            v_paid, v_loading, v_rev)
        returning id into v_sale;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        v_quantity := (v_line ->> 'quantity')::integer;
        v_price := (v_line ->> 'unit_price_minor')::bigint;
        insert into public.sale_lines (sale_id, position, product_id, quantity, unit_price_minor,
                amount_minor, cost_minor)
            values (v_sale, v_position, (v_line ->> 'product_id')::bigint, v_quantity, v_price,
                v_price * v_quantity,
                alm_private.take_lots((v_line ->> 'product_id')::bigint, v_quantity));
        update public.products set stock = stock - v_quantity, rev = v_rev
            where id = (v_line ->> 'product_id')::bigint;
        v_position := v_position + 1;
    end loop;
    v_receipt := jsonb_build_object(
        'draft_id', v_draft, 'invoice', v_invoice, 'quantity', v_units, 'total_minor', v_total);
    insert into public.sale_requests (request_id, terms, sale_id, receipt)
        values (p_request_id, p_terms, v_sale, v_receipt);
    return v_receipt;
end;
$$;

-- {"draft_id": text, "cancel": bool}: cancels (returns stock) or restores (takes it again).
create function public.set_sale_cancelled(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_sale public.sales%rowtype;
    v_cancel boolean := (p_terms ->> 'cancel')::boolean;
    v_line record;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'set_sale_cancelled', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    select * into v_sale from public.sales where draft_id = p_terms ->> 'draft_id' for update;
    if not found or v_cancel is null then
        raise exception 'not_found: sale' using errcode = 'ALM07';
    end if;
    if v_sale.cancelled = v_cancel then
        raise exception 'invalid_state' using errcode = 'ALM08';
    end if;
    v_rev := alm_private.bump();
    for v_line in select * from public.sale_lines where sale_id = v_sale.id order by product_id loop
        if v_cancel then
            update public.products set stock = stock + v_line.quantity, rev = v_rev
                where id = v_line.product_id;
            insert into public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                values (v_line.product_id, v_sale.sale_date, v_line.quantity,
                    v_line.cost_minor / v_line.quantity);
        else
            if (select stock from public.products where id = v_line.product_id for update)
                < v_line.quantity then
                raise exception 'insufficient_stock' using errcode = 'ALM04';
            end if;
            perform alm_private.take_lots(v_line.product_id, v_line.quantity);
            update public.products set stock = stock - v_line.quantity, rev = v_rev
                where id = v_line.product_id;
        end if;
    end loop;
    update public.sales set cancelled = v_cancel, rev = v_rev where id = v_sale.id;
    return alm_private.record(p_request_id, 'set_sale_cancelled', p_terms, '{"done": true}');
end;
$$;

-- {"draft_id": text, "lines": [{"quantity", "rate_minor"}]}: one entry per existing line.
create function public.edit_sale(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_sale public.sales%rowtype;
    v_line record;
    v_edit jsonb;
    v_quantity integer;
    v_rate bigint;
    v_total bigint := 0;
    v_units bigint := 0;
    v_rev bigint;
    v_outcome jsonb;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'edit_sale', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    select * into v_sale from public.sales where draft_id = p_terms ->> 'draft_id' for update;
    if not found then
        raise exception 'not_found: sale' using errcode = 'ALM07';
    end if;
    if v_sale.cancelled then
        raise exception 'invalid_state' using errcode = 'ALM08';
    end if;
    if jsonb_typeof(p_terms -> 'lines') is distinct from 'array' or jsonb_array_length(p_terms -> 'lines')
        <> (select count(*) from public.sale_lines where sale_id = v_sale.id) then
        perform alm_private.invalid('lines');
    end if;
    v_rev := alm_private.bump();
    for v_line in select * from public.sale_lines where sale_id = v_sale.id order by position loop
        v_edit := p_terms -> 'lines' -> v_line.position;
        v_quantity := (v_edit ->> 'quantity')::integer;
        v_rate := (v_edit ->> 'rate_minor')::bigint;
        if v_quantity is null or v_quantity <= 0 or v_rate is null or v_rate < 0 then
            perform alm_private.invalid('line');
        end if;
        if v_quantity > v_line.quantity then
            if (select stock from public.products where id = v_line.product_id for update)
                < v_quantity - v_line.quantity then
                raise exception 'insufficient_stock' using errcode = 'ALM04';
            end if;
            perform alm_private.take_lots(v_line.product_id, v_quantity - v_line.quantity);
        elsif v_quantity < v_line.quantity then
            insert into public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                values (v_line.product_id, v_sale.sale_date, v_line.quantity - v_quantity, null);
        end if;
        update public.products set stock = stock - (v_quantity - v_line.quantity), rev = v_rev
            where id = v_line.product_id;
        update public.sale_lines set quantity = v_quantity, unit_price_minor = v_rate,
                amount_minor = v_quantity * v_rate
            where sale_id = v_sale.id and position = v_line.position;
        v_total := v_total + v_quantity * v_rate;
        v_units := v_units + v_quantity;
    end loop;
    v_total := v_total + v_sale.loading_minor * v_units;
    if v_total < v_sale.paid_minor then
        raise exception 'invalid_terms: below paid' using errcode = 'ALM05';
    end if;
    update public.sales set total_minor = v_total, rev = v_rev where id = v_sale.id;
    v_outcome := jsonb_build_object('draft_id', v_sale.draft_id, 'invoice', v_sale.invoice_no,
        'quantity', v_units, 'total_minor', v_total);
    return alm_private.record(p_request_id, 'edit_sale', p_terms, v_outcome);
end;
$$;

-- {"customer_id", "date", "amount_minor", "online"}.
create function public.record_receipt(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_amount bigint := (p_terms ->> 'amount_minor')::bigint;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'record_receipt', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.customers where id = p_terms ->> 'customer_id') then
        raise exception 'not_found: customer' using errcode = 'ALM07';
    end if;
    if v_amount is null or v_amount = 0 or p_terms -> 'online' is null then
        perform alm_private.invalid('amount');
    end if;
    v_rev := alm_private.bump();
    insert into public.receipts (customer_id, receipt_date, amount_minor, online, rev)
        values (p_terms ->> 'customer_id', alm_private.text_date(p_terms -> 'date'), v_amount,
            (p_terms ->> 'online')::boolean, v_rev);
    return alm_private.record(p_request_id, 'record_receipt', p_terms, '{"done": true}');
end;
$$;

-- {"id": null|n, "date", "amounts": {"cash", "paytm", "phonepe", "goods", "misc", "salary",
-- "debt"}} in paise; "delete": true removes row `id`.
create function public.save_cash_day(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_a jsonb := p_terms -> 'amounts';
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_cash_day', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_id is not null and not exists (select 1 from public.cash_days where id = v_id) then
        raise exception 'not_found: cash day' using errcode = 'ALM07';
    end if;
    if coalesce((p_terms ->> 'delete')::boolean, false) then
        if v_id is null then
            perform alm_private.invalid('id');
        end if;
        v_rev := alm_private.bump();
        delete from public.cash_days where id = v_id;
        perform alm_private.removed(v_rev, 'cash_day', v_id::text);
        return alm_private.record(p_request_id, 'save_cash_day', p_terms, '{"done": true}');
    end if;
    if v_a is null or alm_private.text_date(p_terms -> 'date') is null then
        perform alm_private.invalid('date');
    end if;
    v_rev := alm_private.bump();
    if v_id is null then
        insert into public.cash_days (day, cash_minor, paytm_minor, phonepe_minor, goods_minor,
                misc_minor, salary_minor, debt_minor, rev)
            values (alm_private.text_date(p_terms -> 'date'), (v_a ->> 'cash')::bigint,
                (v_a ->> 'paytm')::bigint, (v_a ->> 'phonepe')::bigint, (v_a ->> 'goods')::bigint,
                (v_a ->> 'misc')::bigint, (v_a ->> 'salary')::bigint, (v_a ->> 'debt')::bigint,
                v_rev);
    else
        update public.cash_days set day = alm_private.text_date(p_terms -> 'date'),
                cash_minor = (v_a ->> 'cash')::bigint, paytm_minor = (v_a ->> 'paytm')::bigint,
                phonepe_minor = (v_a ->> 'phonepe')::bigint,
                goods_minor = (v_a ->> 'goods')::bigint, misc_minor = (v_a ->> 'misc')::bigint,
                salary_minor = (v_a ->> 'salary')::bigint, debt_minor = (v_a ->> 'debt')::bigint,
                rev = v_rev
            where id = v_id;
    end if;
    return alm_private.record(p_request_id, 'save_cash_day', p_terms, '{"done": true}');
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Catalog and purchasing.

-- {"id", "new", "item", "brand", "buy_minor", "sell_minor", "reorder": null|n,
--  "reviewed": null|{"buy_minor", "sell_minor"}}.
create function public.save_product(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_item text := btrim(coalesce(p_terms ->> 'item', ''));
    v_new boolean := (p_terms ->> 'new')::boolean;
    v_product public.products%rowtype;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_product', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_id is null or v_new is null or (p_terms ->> 'sell_minor')::bigint < 0
        or (p_terms ->> 'buy_minor')::bigint < 0 then
        perform alm_private.invalid('product');
    end if;
    if v_item = '' then
        perform alm_private.invalid('item name required');
    end if;
    select * into v_product from public.products where id = v_id for update;
    if v_new and found then
        raise exception 'invalid_state: id in use' using errcode = 'ALM08';
    end if;
    if not v_new and not found then
        raise exception 'not_found: product' using errcode = 'ALM07';
    end if;
    if not v_new and p_terms -> 'reviewed' is not null and jsonb_typeof(p_terms -> 'reviewed') <> 'null'
        and ((p_terms #>> '{reviewed,buy_minor}')::bigint <> v_product.buy_minor
            or (p_terms #>> '{reviewed,sell_minor}')::bigint <> v_product.sell_minor) then
        raise exception 'stale_review' using errcode = 'ALM03';
    end if;
    if exists (select 1 from public.products where id <> v_id and lower(item) = lower(v_item)) then
        perform alm_private.invalid('duplicate name');
    end if;
    v_rev := alm_private.bump();
    insert into public.products (id, item, brand, buy_minor, sell_minor, stock, reorder_level, rev)
        values (v_id, v_item, btrim(coalesce(p_terms ->> 'brand', '')),
            (p_terms ->> 'buy_minor')::bigint, (p_terms ->> 'sell_minor')::bigint, 0,
            coalesce((p_terms ->> 'reorder')::integer, 5), v_rev)
        on conflict (id) do update set item = excluded.item, brand = excluded.brand,
            buy_minor = excluded.buy_minor, sell_minor = excluded.sell_minor,
            reorder_level = excluded.reorder_level, rev = excluded.rev;
    return alm_private.record(p_request_id, 'save_product', p_terms, '{"done": true}');
end;
$$;

-- {"id"}: refused for a product on a posted bill, as in the app.
create function public.delete_product(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'delete_product', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.products where id = v_id) then
        raise exception 'not_found: product' using errcode = 'ALM07';
    end if;
    if exists (select 1 from public.sale_lines where product_id = v_id) then
        raise exception 'invalid_state: product in use' using errcode = 'ALM08';
    end if;
    v_rev := alm_private.bump();
    delete from public.products where id = v_id;
    perform alm_private.removed(v_rev, 'product', v_id::text);
    return alm_private.record(p_request_id, 'delete_product', p_terms, '{"done": true}');
end;
$$;

-- {"name"}: a new comparison supplier; returns {"created": id}.
create function public.add_supplier(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_name text := btrim(coalesce(p_terms ->> 'name', ''));
    v_id bigint;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'add_supplier', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    perform pg_advisory_xact_lock(hashtextextended('suppliers', 0));
    if v_name = '' or exists (select 1 from public.suppliers where lower(name) = lower(v_name)) then
        perform alm_private.invalid('Enter a new supplier name.');
    end if;
    v_rev := alm_private.bump();
    select coalesce(max(id) + 1, 0) into v_id from public.suppliers;
    insert into public.suppliers (id, name, rev) values (v_id, v_name, v_rev);
    return alm_private.record(p_request_id, 'add_supplier', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

-- {"product_id", "rates": [{"supplier_id", "rate_minor": null|n}]}: null or 0 clears.
create function public.set_supplier_prices(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_product bigint := (p_terms ->> 'product_id')::bigint;
    v_rate jsonb;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'set_supplier_prices', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.products where id = v_product) then
        raise exception 'not_found: product' using errcode = 'ALM07';
    end if;
    v_rev := alm_private.bump();
    for v_rate in select * from jsonb_array_elements(coalesce(p_terms -> 'rates', '[]')) loop
        if not exists (select 1 from public.suppliers where id = (v_rate ->> 'supplier_id')::bigint) then
            raise exception 'not_found: supplier' using errcode = 'ALM07';
        end if;
        if coalesce((v_rate ->> 'rate_minor')::bigint, 0) = 0 then
            delete from public.supplier_prices
                where product_id = v_product and supplier_id = (v_rate ->> 'supplier_id')::bigint;
            perform alm_private.removed(v_rev, 'supplier_price',
                v_product || ':' || (v_rate ->> 'supplier_id'));
        elsif (v_rate ->> 'rate_minor')::bigint < 0 then
            perform alm_private.invalid('rate');
        else
            insert into public.supplier_prices values (v_product,
                    (v_rate ->> 'supplier_id')::bigint, (v_rate ->> 'rate_minor')::bigint, v_rev)
                on conflict (product_id, supplier_id)
                do update set rate_minor = excluded.rate_minor, rev = excluded.rev;
        end if;
    end loop;
    return alm_private.record(p_request_id, 'set_supplier_prices', p_terms, '{"done": true}');
end;
$$;

-- {"supplier": null|{"id": null|n, "name"}, "date", "note", "lines": [{"item",
-- "product_id": null|n, "ordered", "rate_minor"}]}: returns {"created": po id}.
create function public.save_purchase_order(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_name text := btrim(coalesce(p_terms #>> '{supplier,name}', ''));
    v_supplier bigint;
    v_id bigint;
    v_line jsonb;
    v_position integer := 0;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_purchase_order', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_name = '' then
        perform alm_private.invalid('Enter supplier');
    end if;
    if not exists (select 1 from jsonb_array_elements(coalesce(p_terms -> 'lines', '[]')) l
            where btrim(coalesce(l ->> 'item', '')) <> '') then
        perform alm_private.invalid('Add at least one item with an order quantity');
    end if;
    if alm_private.text_date(p_terms -> 'date') is null then
        perform alm_private.invalid('Pick the order date');
    end if;
    perform pg_advisory_xact_lock(hashtextextended('suppliers', 0));
    v_rev := alm_private.bump();
    select id into v_supplier from public.suppliers where lower(name) = lower(v_name);
    if not found then
        select coalesce(max(id) + 1, 0) into v_supplier from public.suppliers;
        insert into public.suppliers (id, name, rev) values (v_supplier, v_name, v_rev);
    end if;
    select coalesce(max(id) + 1, 0) into v_id from public.purchase_orders;
    insert into public.purchase_orders (id, supplier_id, supplier_name, order_date, note, rev)
        values (v_id, v_supplier, v_name, alm_private.text_date(p_terms -> 'date'),
            coalesce(p_terms ->> 'note', ''), v_rev);
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        continue when btrim(coalesce(v_line ->> 'item', '')) = '';
        if coalesce((v_line ->> 'ordered')::integer, 0) <= 0 or (v_line ->> 'rate_minor')::bigint < 0 then
            perform alm_private.invalid('line');
        end if;
        insert into public.purchase_lines (po_id, position, item, product_id, ordered, rate_minor)
            values (v_id, v_position, v_line ->> 'item',
                coalesce((v_line ->> 'product_id')::bigint,
                    (select id from public.products where item = v_line ->> 'item')),
                (v_line ->> 'ordered')::integer, (v_line ->> 'rate_minor')::bigint);
        v_position := v_position + 1;
    end loop;
    return alm_private.record(p_request_id, 'save_purchase_order', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

-- Rejects as stale unless each line's remaining quantity equals the reviewed one.
create function alm_private.check_remaining(p_po bigint, p_remaining jsonb)
returns void
language plpgsql
set search_path = ''
as $$
begin
    if not exists (select 1 from public.purchase_orders where id = p_po) then
        raise exception 'not_found: purchase order' using errcode = 'ALM07';
    end if;
    perform 1 from public.purchase_lines where po_id = p_po for update;
    if coalesce((select jsonb_agg(ordered - received - closed order by position)
            from public.purchase_lines where po_id = p_po), '[]') <> coalesce(p_remaining, '[]') then
        raise exception 'stale_review' using errcode = 'ALM03';
    end if;
end;
$$;

-- {"po", "date": null|d, "today", "lines": [{"quantity", "rate_minor"}], "remaining": [n]}.
create function public.receive_purchase_order(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_po bigint := (p_terms ->> 'po')::bigint;
    v_line record;
    v_input jsonb;
    v_quantity integer;
    v_rate bigint;
    v_any boolean := false;
    v_day date;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'receive_purchase_order', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    perform alm_private.check_remaining(v_po, p_terms -> 'remaining');
    if jsonb_array_length(coalesce(p_terms -> 'lines', '[]'))
        <> (select count(*) from public.purchase_lines where po_id = v_po) then
        perform alm_private.invalid('Invalid receipt');
    end if;
    v_day := coalesce(alm_private.text_date(p_terms -> 'date'), alm_private.text_date(p_terms -> 'today'));
    v_rev := alm_private.bump();
    for v_line in select * from public.purchase_lines where po_id = v_po order by position loop
        v_input := p_terms -> 'lines' -> v_line.position;
        v_quantity := least(coalesce((v_input ->> 'quantity')::integer, 0),
            v_line.ordered - v_line.received - v_line.closed);
        v_rate := coalesce((v_input ->> 'rate_minor')::bigint, 0);
        if v_quantity < 0 or v_rate < 0 then
            perform alm_private.invalid('Invalid receipt');
        end if;
        continue when v_quantity = 0;
        v_any := true;
        if v_rate = 0 then
            v_rate := v_line.rate_minor;
        end if;
        update public.purchase_lines set received = received + v_quantity, rate_minor = v_rate
            where po_id = v_po and position = v_line.position;
        if v_line.product_id is not null
            and exists (select 1 from public.products where id = v_line.product_id) then
            update public.products set stock = stock + v_quantity, rev = v_rev
                where id = v_line.product_id;
            insert into public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                values (v_line.product_id, v_day, v_quantity, v_rate);
        end if;
    end loop;
    if not v_any then
        perform alm_private.invalid('Enter at least one quantity to receive');
    end if;
    update public.purchase_orders set rev = v_rev where id = v_po;
    return alm_private.record(p_request_id, 'receive_purchase_order', p_terms, '{"done": true}');
end;
$$;

-- {"po", "remaining": [n]}: "Close remaining".
create function public.close_purchase_order(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_po bigint := (p_terms ->> 'po')::bigint;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'close_purchase_order', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    perform alm_private.check_remaining(v_po, p_terms -> 'remaining');
    v_rev := alm_private.bump();
    update public.purchase_lines set closed = ordered - received where po_id = v_po;
    update public.purchase_orders set rev = v_rev where id = v_po;
    return alm_private.record(p_request_id, 'close_purchase_order', p_terms, '{"done": true}');
end;
$$;

-- {"id": null|n, "name", "price_minor", "components": [{"product_id", "quantity", "piece"}]}.
create function public.save_crate(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_c jsonb;
    v_position integer := 0;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_crate', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if btrim(coalesce(p_terms ->> 'name', '')) = ''
        or jsonb_array_length(coalesce(p_terms -> 'components', '[]')) = 0
        or coalesce((p_terms ->> 'price_minor')::bigint, -1) < 0 then
        perform alm_private.invalid('Empty');
    end if;
    if v_id is not null and not exists (select 1 from public.crate_templates where id = v_id) then
        raise exception 'not_found: crate' using errcode = 'ALM07';
    end if;
    v_rev := alm_private.bump();
    if v_id is null then
        select coalesce(max(id), 0) + 1 into v_id from public.crate_templates;
        insert into public.crate_templates values (v_id, btrim(p_terms ->> 'name'),
            (p_terms ->> 'price_minor')::bigint, v_rev);
    else
        update public.crate_templates set name = btrim(p_terms ->> 'name'),
            price_minor = (p_terms ->> 'price_minor')::bigint, rev = v_rev where id = v_id;
        delete from public.crate_components where crate_id = v_id;
    end if;
    for v_c in select * from jsonb_array_elements(p_terms -> 'components') loop
        if coalesce((v_c ->> 'quantity')::integer, 0) <= 0
            or not exists (select 1 from public.products where id = (v_c ->> 'product_id')::bigint) then
            perform alm_private.invalid('component');
        end if;
        insert into public.crate_components values (v_id, v_position,
            (v_c ->> 'product_id')::bigint, (v_c ->> 'quantity')::integer,
            coalesce((v_c ->> 'piece')::boolean, false));
        v_position := v_position + 1;
    end loop;
    return alm_private.record(p_request_id, 'save_crate', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

create function public.delete_crate(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'delete_crate', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.crate_templates where id = v_id) then
        raise exception 'not_found: crate' using errcode = 'ALM07';
    end if;
    v_rev := alm_private.bump();
    delete from public.crate_templates where id = v_id;
    perform alm_private.removed(v_rev, 'crate', v_id::text);
    return alm_private.record(p_request_id, 'delete_crate', p_terms, '{"done": true}');
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Customers, godown and stock counts.

-- {"customer_id", "amount_minor": null|n (negative = advance), "date": null|d}.
create function public.save_opening(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_opening', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.customers where id = p_terms ->> 'customer_id') then
        raise exception 'not_found: customer' using errcode = 'ALM07';
    end if;
    v_rev := alm_private.bump();
    update public.customers set opening_minor = coalesce((p_terms ->> 'amount_minor')::bigint, 0),
            opening_date = alm_private.text_date(p_terms -> 'date'), rev = v_rev
        where id = p_terms ->> 'customer_id';
    return alm_private.record(p_request_id, 'save_opening', p_terms, '{"done": true}');
end;
$$;

-- {"item", "cases" (whole decimal cases as typed), "note"}: upsert by case-insensitive name;
-- a blank note keeps the previous one.
create function public.save_godown(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_item text := btrim(coalesce(p_terms ->> 'item', ''));
    v_cases text := btrim(coalesce(p_terms ->> 'cases', ''));
    v_note text := nullif(btrim(coalesce(p_terms ->> 'note', '')), '');
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_godown', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_item = '' then
        perform alm_private.invalid('Type an item name');
    end if;
    if v_cases !~ '^[0-9]{1,9}$' then
        perform alm_private.invalid('Enter quantity (whole cases)');
    end if;
    v_rev := alm_private.bump();
    insert into public.godown_rows values (lower(v_item), v_item, v_cases::integer, v_note, v_rev)
        on conflict (key) do update set cases = excluded.cases,
            note = coalesce(excluded.note, public.godown_rows.note), rev = excluded.rev;
    return alm_private.record(p_request_id, 'save_godown', p_terms, '{"done": true}');
end;
$$;

-- {"product_id", "counted", "fridge": null|n, "date", "reason": null|text, "note": null|text,
--  "on_hand"}: stale unless on_hand is the current stock.
create function public.count_stock(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_product public.products%rowtype;
    v_counted integer := (p_terms ->> 'counted')::integer;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'count_stock', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    select * into v_product from public.products where id = (p_terms ->> 'product_id')::bigint
        for update;
    if not found then
        raise exception 'not_found: product' using errcode = 'ALM07';
    end if;
    if v_counted is null or v_counted < 0 or coalesce((p_terms ->> 'fridge')::integer, 0) < 0 then
        perform alm_private.invalid('count');
    end if;
    if (p_terms ->> 'on_hand')::integer is distinct from v_product.stock then
        raise exception 'stale_review' using errcode = 'ALM03';
    end if;
    v_rev := alm_private.bump();
    if v_counted <> v_product.stock then
        insert into public.stock_counts (product_id, count_date, from_qty, to_qty, reason, note)
            values (v_product.id, alm_private.text_date(p_terms -> 'date'), v_product.stock,
                v_counted, p_terms ->> 'reason', p_terms ->> 'note');
        if v_counted > v_product.stock then
            insert into public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                values (v_product.id, alm_private.text_date(p_terms -> 'date'),
                    v_counted - v_product.stock, null);
        else
            perform alm_private.take_lots(v_product.id, v_product.stock - v_counted);
        end if;
    end if;
    update public.products set stock = v_counted,
            fridge = coalesce((p_terms ->> 'fridge')::integer, fridge), rev = v_rev
        where id = v_product.id;
    return alm_private.record(p_request_id, 'count_stock', p_terms, '{"done": true}');
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Standalone GST bills: numbers allocated here, bodies saved under the allocated number.

create or replace function public.allocate_gst_bill_number(p_bill_id text)
returns bigint
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_number bigint;
begin
    perform alm_private.require_owner();
    if p_bill_id is null or length(p_bill_id) not between 1 and 64 then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('gst_bill_numbers', 0));
    select number into v_number from public.gst_bills where id = p_bill_id;
    if found then
        return v_number;
    end if;
    if exists (select 1 from public.gst_deleted where id = p_bill_id) then
        raise exception 'invalid_state: deleted' using errcode = 'ALM08';
    end if;
    select coalesce(max(number), 0) + 1 into v_number from public.gst_bills;
    insert into public.gst_bills (id, number, rev) values (p_bill_id, v_number, alm_private.bump());
    return v_number;
end;
$$;

-- {"bill": <GstBill record JSON: id, number, customer, date, cancelled, lines, …>}.
create function public.save_gst_bill(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_bill jsonb := p_terms -> 'bill';
    v_key text := 'gst-' || (p_terms #>> '{bill,id}');
    v_row public.gst_bills%rowtype;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'save_gst_bill', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_bill is null or btrim(coalesce(v_bill ->> 'customer', '')) = ''
        or jsonb_array_length(coalesce(v_bill -> 'lines', '[]')) = 0 then
        perform alm_private.invalid('bill');
    end if;
    if exists (select 1 from public.gst_deleted where id = v_key) then
        raise exception 'invalid_state: deleted' using errcode = 'ALM08';
    end if;
    select * into v_row from public.gst_bills where id = v_key for update;
    if not found or v_row.number <> (v_bill ->> 'number')::bigint then
        raise exception 'stale_review' using errcode = 'ALM03';
    end if;
    if v_row.cancelled or coalesce((v_bill ->> 'cancelled')::boolean, false) then
        raise exception 'invalid_state: cancelled' using errcode = 'ALM08';
    end if;
    v_rev := alm_private.bump();
    update public.gst_bills set body = v_bill, rev = v_rev where id = v_key;
    return alm_private.record(p_request_id, 'save_gst_bill', p_terms,
        jsonb_build_object('gst_number', v_row.number));
end;
$$;

-- {"id", "delete": bool}: cancel keeps the bill and its number; delete frees the number.
create function public.set_gst_bill_state(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_key text := 'gst-' || (p_terms ->> 'id');
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'set_gst_bill_state', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if not exists (select 1 from public.gst_bills where id = v_key and body is not null) then
        raise exception 'not_found: gst bill' using errcode = 'ALM07';
    end if;
    v_rev := alm_private.bump();
    if coalesce((p_terms ->> 'delete')::boolean, false) then
        delete from public.gst_bills where id = v_key;
        insert into public.gst_deleted values (v_key) on conflict do nothing;
        perform alm_private.removed(v_rev, 'gst_bill', v_key);
    else
        update public.gst_bills set cancelled = true,
            body = jsonb_set(body, '{cancelled}', 'true'), rev = v_rev where id = v_key;
    end if;
    return alm_private.record(p_request_id, 'set_gst_bill_state', p_terms, '{"done": true}');
end;
$$;

-- {"bill": null|record, "deleted": null|id}: moves a device-stored bill or deletion marker in
-- unchanged; an identical bill already present is a no-op, a different one is refused.
create function public.import_gst_bill(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_bill jsonb := p_terms -> 'bill';
    v_key text;
    v_row public.gst_bills%rowtype;
    v_rev bigint;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'import_gst_bill', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    perform pg_advisory_xact_lock(hashtextextended('gst_bill_numbers', 0));
    v_rev := alm_private.bump();
    if v_bill is not null and jsonb_typeof(v_bill) = 'object' then
        v_key := 'gst-' || (v_bill ->> 'id');
        select * into v_row from public.gst_bills where id = v_key;
        if found and v_row.body is not distinct from v_bill then
            null;
        elsif found or exists (select 1 from public.gst_deleted where id = v_key)
            or exists (select 1 from public.gst_bills where number = (v_bill ->> 'number')::bigint) then
            raise exception 'invalid_state: conflict' using errcode = 'ALM08';
        else
            insert into public.gst_bills (id, number, cancelled, body, rev)
                values (v_key, (v_bill ->> 'number')::bigint,
                    coalesce((v_bill ->> 'cancelled')::boolean, false), v_bill, v_rev);
        end if;
    end if;
    if p_terms ->> 'deleted' is not null then
        v_key := 'gst-' || (p_terms ->> 'deleted');
        if exists (select 1 from public.gst_bills where id = v_key) then
            raise exception 'invalid_state: conflict' using errcode = 'ALM08';
        end if;
        insert into public.gst_deleted values (v_key) on conflict do nothing;
    end if;
    return alm_private.record(p_request_id, 'import_gst_bill', p_terms, '{"done": true}');
end;
$$;

-- ---------------------------------------------------------------------------------------
-- Status and projection.

-- The committed outcome of a request, or null. Owner only (OPEN-004: who may check status).
create function public.request_status(p_request_id text)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
begin
    perform alm_private.require_owner();
    return coalesce(
        (select jsonb_build_object('command', command, 'outcome', outcome)
            from public.command_requests where request_id = p_request_id),
        (select jsonb_build_object('command', 'post_sale', 'outcome', receipt)
            from public.sale_requests where request_id = p_request_id));
end;
$$;

-- Role-filtered rows changed since `p_checkpoint` (all rows when null), the removals since then
-- and the new checkpoint. A Manager receives only the catalog it drafts with (id, name, sell
-- rate): no stock, balances, buy rates, quotes, purchase orders, lots, costs or margins.
create function public.pull_projection(p_checkpoint bigint)
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
    v_rows := jsonb_build_object('catalog', coalesce((select jsonb_agg(jsonb_build_object(
            'id', id, 'name', item, 'rate', sell_minor) order by id)
        from public.products where rev > v_since), '[]'));
    if v_role = 'owner' then
        v_rows := v_rows || jsonb_build_object(
            'products', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'item', item,
                    'brand', brand, 'sell', sell_minor, 'reorder_level', reorder_level,
                    'on_hand', stock) order by id)
                from public.products where rev > v_since), '[]'),
            'customers', coalesce((select jsonb_agg(jsonb_build_object('id', c.id, 'name', c.name,
                    'balance', c.opening_minor
                        + coalesce((select sum(total_minor - paid_minor) from public.sales s
                            where s.customer_id = c.id and not s.cancelled), 0)
                        - coalesce((select sum(amount_minor) from public.receipts r
                            where r.customer_id = c.id), 0)) order by c.name)
                from public.customers c), '[]'),
            'sales', coalesce((select jsonb_agg(jsonb_build_object('draft_id', draft_id,
                    'invoice', invoice_no, 'date', sale_date, 'customer', customer_name,
                    'lines', (select count(*) from public.sale_lines l where l.sale_id = s.id),
                    'packets', (select coalesce(sum(quantity), 0) from public.sale_lines l
                        where l.sale_id = s.id),
                    'total', total_minor, 'paid', paid_minor, 'cancelled', cancelled)
                    order by invoice_no)
                from public.sales s where rev > v_since), '[]'),
            'receipts', coalesce((select jsonb_agg(jsonb_build_object('slot', id,
                    'date', receipt_date, 'customer', customer_id, 'amount', amount_minor,
                    'online', online) order by id)
                from public.receipts where rev > v_since), '[]'),
            'cash_days', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'date', day,
                    'cash', cash_minor, 'paytm', paytm_minor, 'phonepe', phonepe_minor,
                    'goods', goods_minor, 'misc', misc_minor, 'salary', salary_minor,
                    'debt', debt_minor) order by id)
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
                        order by id) from public.products where rev > v_since), '[]'),
                'supplier_prices', coalesce((select jsonb_agg(jsonb_build_array(product_id,
                        supplier_id, rate_minor) order by product_id, supplier_id)
                    from public.supplier_prices where rev > v_since), '[]'),
                'purchase_orders', coalesce((select jsonb_agg(jsonb_build_object('id', o.id,
                        'supplier', o.supplier_name, 'date', o.order_date,
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
                        (select case when bool_and(cost_minor is not null) then sum(cost_minor) end
                            from public.sale_lines l where l.sale_id = s.id))
                        order by s.invoice_no)
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

-- ---------------------------------------------------------------------------------------
-- Deny by default: no table, sequence or function is reachable by client roles except the
-- command, status and pull functions, whose bodies check the JWT role.

revoke all on all tables in schema public from anon, authenticated;
revoke all on all sequences in schema public from anon, authenticated;
revoke all on all functions in schema public from public, anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;

do $$
declare
    f text;
begin
    foreach f in array array[
        'post_sale(text, jsonb)', 'set_sale_cancelled(text, jsonb)', 'edit_sale(text, jsonb)',
        'record_receipt(text, jsonb)', 'save_cash_day(text, jsonb)', 'save_product(text, jsonb)',
        'delete_product(text, jsonb)', 'add_supplier(text, jsonb)',
        'set_supplier_prices(text, jsonb)', 'save_purchase_order(text, jsonb)',
        'receive_purchase_order(text, jsonb)', 'close_purchase_order(text, jsonb)',
        'save_crate(text, jsonb)', 'delete_crate(text, jsonb)', 'save_opening(text, jsonb)',
        'save_godown(text, jsonb)', 'count_stock(text, jsonb)', 'allocate_gst_bill_number(text)',
        'save_gst_bill(text, jsonb)', 'set_gst_bill_state(text, jsonb)',
        'import_gst_bill(text, jsonb)', 'request_status(text)', 'pull_projection(bigint)'
    ] loop
        execute format('grant execute on function public.%s to authenticated, service_role', f);
    end loop;
end;
$$;
