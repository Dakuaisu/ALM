-- Local development schema for the synthetic ALM slice. Not the deployed schema (OPEN-002);
-- final permissions wait on OPEN-004, so every table starts deny-by-default.

create table public.products (
    id bigint primary key check (id >= 0),
    item text not null,
    sell_minor bigint not null check (sell_minor >= 0),
    stock integer not null check (stock >= 0)
);

-- One row per numbering series: 'sale' for ordinary invoices.
create table public.invoice_counters (
    series text primary key,
    next_no bigint not null check (next_no >= 1)
);
insert into public.invoice_counters (series, next_no) values ('sale', 1);

create table public.sales (
    id bigint generated always as identity primary key,
    invoice_no bigint not null unique,
    draft_id text not null unique,
    total_minor bigint not null check (total_minor >= 0),
    posted_at timestamptz not null default now()
);

create table public.sale_lines (
    sale_id bigint not null references public.sales (id),
    position integer not null check (position >= 0),
    product_id bigint not null references public.products (id),
    quantity integer not null check (quantity > 0),
    unit_price_minor bigint not null check (unit_price_minor >= 0),
    amount_minor bigint not null check (amount_minor >= 0),
    primary key (sale_id, position)
);

-- The idempotency record: the exact terms a request ID was first used with and its receipt.
create table public.sale_requests (
    request_id text primary key check (length(request_id) between 1 and 64),
    terms jsonb not null,
    sale_id bigint not null references public.sales (id),
    receipt jsonb not null,
    created_at timestamptz not null default now()
);

-- Standalone GST bills; numbers are allocated here so two devices cannot issue the same one.
create table public.gst_bills (
    id text primary key check (length(id) between 1 and 64),
    number bigint not null check (number >= 1),
    cancelled boolean not null default false,
    body jsonb,
    created_at timestamptz not null default now()
);

alter table public.products enable row level security;
alter table public.invoice_counters enable row level security;
alter table public.sales enable row level security;
alter table public.sale_lines enable row level security;
alter table public.sale_requests enable row level security;
alter table public.gst_bills enable row level security;
-- No policies yet: anon and authenticated see and change nothing (OPEN-004).
revoke all on all tables in schema public from anon, authenticated;

-- Posts one sale atomically. p_terms = {"draft_id": text, "lines": [{"product_id", "quantity",
-- "unit_price_minor"}]}. A retry with identical terms returns the original receipt; a reused
-- request ID with different terms raises request_conflict (SQLSTATE ALM01).
create function public.post_sale(p_request_id text, p_terms jsonb)
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
    v_invoice bigint;
    v_sale bigint;
    v_receipt jsonb;
begin
    if p_request_id is null or v_draft is null or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(p_terms -> 'lines') = 0 then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    -- Serialize every use of this request ID before looking it up.
    perform pg_advisory_xact_lock(hashtextextended('sale_request:' || p_request_id, 0));
    select * into v_existing from public.sale_requests where request_id = p_request_id;
    if found then
        if v_existing.terms = p_terms then
            return v_existing.receipt;
        end if;
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('sale_draft:' || v_draft, 0));
    if exists (select 1 from public.sales where draft_id = v_draft) then
        raise exception 'draft_already_posted' using errcode = 'ALM02';
    end if;
    -- Lock products in ID order so concurrent sales cannot deadlock.
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
    update public.invoice_counters set next_no = next_no + 1
        where series = 'sale' returning next_no - 1 into v_invoice;
    insert into public.sales (invoice_no, draft_id, total_minor)
        values (v_invoice, v_draft, v_total) returning id into v_sale;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        v_quantity := (v_line ->> 'quantity')::integer;
        v_price := (v_line ->> 'unit_price_minor')::bigint;
        insert into public.sale_lines (sale_id, position, product_id, quantity, unit_price_minor, amount_minor)
            values (v_sale, v_position, (v_line ->> 'product_id')::bigint, v_quantity, v_price, v_price * v_quantity);
        update public.products set stock = stock - v_quantity
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

-- nextGstNo, made central: the highest stored GST number (cancelled included) plus one, under a
-- lock so concurrent devices get distinct numbers. Idempotent per bill ID. Deleting the
-- highest-numbered bill frees its number again, as in the HTML.
create function public.allocate_gst_bill_number(p_bill_id text)
returns bigint
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_number bigint;
begin
    perform pg_advisory_xact_lock(hashtextextended('gst_bill_numbers', 0));
    select number into v_number from public.gst_bills where id = p_bill_id;
    if found then
        return v_number;
    end if;
    select coalesce(max(number), 0) + 1 into v_number from public.gst_bills;
    insert into public.gst_bills (id, number) values (p_bill_id, v_number);
    return v_number;
end;
$$;

revoke all on function public.post_sale(text, jsonb) from public, anon, authenticated;
revoke all on function public.allocate_gst_bill_number(text) from public, anon, authenticated;
grant execute on function public.post_sale(text, jsonb) to service_role;
grant execute on function public.allocate_gst_bill_number(text) to service_role;
