-- Local development schema, continued: a bounded number of locks per transaction. Requests
-- and drafts were serialized with one transaction-scoped advisory lock per request ID and per
-- draft ID (`replay`, the `post_sale` bodies, `sell_crate`). Such locks are held until the
-- transaction ends, so a transaction that runs many commands (a batch, a test harness, a
-- migration replay) held one or two locks per command and ran out of the shared lock table
-- (SQLSTATE 53200 "out of shared memory") after 12,000-14,000 of them on a default postgres:17
-- (max_locks_per_transaction 64), i.e. well before 20,000 bills;
-- the lock table cannot be enlarged on hosted Supabase. Each ID now maps to one of 1,024
-- buckets per kind (two-key advisory locks: kind, bucket): requests that share a bucket wait
-- for each other briefly, nothing else changes, and a transaction holds at most 2,048 of
-- these locks whatever it runs. Within one command the request lock is still taken before
-- the draft lock.

-- Takes the transaction lock for `p_key`'s bucket in lock space `p_space`.
create function alm_private.xact_lock(p_space integer, p_key text)
returns void
language sql
set search_path = ''
as $$
    select pg_catalog.pg_advisory_xact_lock(p_space, pg_catalog.hashtext(p_key) & 1023);
$$;

create or replace function alm_private.replay(p_request_id text, p_command text, p_terms jsonb)
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
    perform alm_private.xact_lock(1, 'sale_request:' || p_request_id);
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

create or replace function alm_private.post_sale_lines_body(p_request_id text, p_terms jsonb)
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
    v_line_cost bigint;
    v_cost bigint := 0;
    v_known boolean := true;
    v_receipt jsonb;
begin
    perform alm_private.require_owner();
    if p_request_id is null or v_draft is null or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(p_terms -> 'lines') = 0 or v_loading < 0 then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    perform alm_private.xact_lock(1, 'sale_request:' || p_request_id);
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
    perform alm_private.xact_lock(2, 'sale_draft:' || v_draft);
    if exists (select 1 from public.sales where draft_id = v_draft) then
        raise exception 'draft_already_posted' using errcode = 'ALM02';
    end if;
    perform 1 from public.products
        where id in (select (l ->> 'product_id')::bigint from jsonb_array_elements(p_terms -> 'lines') l)
        order by id for update;
    -- Priced at the current sell rates: unknown products, then stale prices.
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
        v_total := v_total + v_price * v_quantity;
        v_units := v_units + v_quantity;
    end loop;
    v_total := v_total + v_loading * v_units;
    -- The Owner review: the selected directory customer, a bill date, 0 <= paid <= total.
    if p_terms ->> 'customer_id' is null or not exists (select 1 from public.customers
            where id = p_terms ->> 'customer_id' and name = p_terms ->> 'customer') then
        raise exception 'invalid_terms: pick a customer' using errcode = 'ALM05';
    end if;
    if alm_private.text_date(p_terms -> 'date') is null then
        raise exception 'invalid_terms: bill date' using errcode = 'ALM05';
    end if;
    if v_paid < 0 or v_paid > v_total then
        raise exception 'invalid_terms: amount paid' using errcode = 'ALM05';
    end if;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        if (select stock from public.products where id = (v_line ->> 'product_id')::bigint)
            < (v_line ->> 'quantity')::integer then
            raise exception 'insufficient_stock' using errcode = 'ALM04';
        end if;
    end loop;
    v_rev := alm_private.bump();
    update public.invoice_counters set next_no = next_no + 1
        where series = 'sale' returning next_no - 1 into v_invoice;
    insert into public.sales (invoice_no, draft_id, total_minor, customer_id, customer_name,
            sale_date, paid_minor, loading_minor, rev, epoch)
        values (v_invoice, v_draft, v_total, p_terms ->> 'customer_id', p_terms ->> 'customer',
            alm_private.text_date(p_terms -> 'date'), v_paid, v_loading, v_rev, alm_private.move())
        returning id into v_sale;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        v_quantity := (v_line ->> 'quantity')::integer;
        v_price := (v_line ->> 'unit_price_minor')::bigint;
        insert into public.sale_lines (sale_id, position, product_id, quantity, unit_price_minor,
                amount_minor, item)
            values (v_sale, v_position, (v_line ->> 'product_id')::bigint, v_quantity, v_price,
                v_price * v_quantity,
                (select item from public.products where id = (v_line ->> 'product_id')::bigint));
        v_line_cost := alm_private.allocate(v_sale, v_position, (v_line ->> 'product_id')::bigint,
            v_quantity);
        update public.sale_lines set cost_minor = v_line_cost
            where sale_id = v_sale and position = v_position;
        update public.products set rev = v_rev where id = (v_line ->> 'product_id')::bigint;
        if v_line_cost is null then
            v_known := false;
        else
            v_cost := v_cost + v_line_cost;
        end if;
        v_position := v_position + 1;
    end loop;
    update public.sales set cost_minor = case when v_known then v_cost end where id = v_sale;
    v_receipt := jsonb_build_object(
        'draft_id', v_draft, 'invoice', v_invoice, 'quantity', v_units, 'total_minor', v_total);
    insert into public.sale_requests (request_id, terms, sale_id, receipt)
        values (p_request_id, p_terms, v_sale, v_receipt);
    return v_receipt;
end;
$$;

create or replace function alm_private.post_sale_body(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
    v_existing public.sale_requests%rowtype;
    v_draft text := p_terms ->> 'draft_id';
    v_crates jsonb := p_terms -> 'crates';
    v_line jsonb;
    v_row jsonb;
    v_comp jsonb;
    v_product public.products%rowtype;
    v_position integer := 0;
    v_quantity integer;
    v_price bigint;
    v_pack integer;
    v_demand bigint;
    v_demands jsonb := '[]';
    v_packs jsonb := '[]';
    v_total bigint := 0;
    v_units bigint := 0;
    v_loading bigint := coalesce((p_terms ->> 'loading_minor')::bigint, 0);
    v_paid bigint := coalesce((p_terms ->> 'paid_minor')::bigint, 0);
    v_invoice bigint;
    v_sale bigint;
    v_rev bigint;
    v_line_cost bigint;
    v_cost bigint := 0;
    v_known boolean := true;
    v_rows jsonb := '[]';
    v_receipt jsonb;
begin
    if v_crates is null or v_crates = '[]'::jsonb then
        return alm_private.post_sale_lines_body(p_request_id, p_terms);
    end if;
    perform alm_private.require_owner();
    if p_request_id is null or v_draft is null or jsonb_typeof(v_crates) <> 'array'
        or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(p_terms -> 'lines') = 0 or v_loading < 0 then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    perform alm_private.xact_lock(1, 'sale_request:' || p_request_id);
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
    perform alm_private.xact_lock(2, 'sale_draft:' || v_draft);
    if exists (select 1 from public.sales where draft_id = v_draft) then
        raise exception 'draft_already_posted' using errcode = 'ALM02';
    end if;
    -- Crate rows carry their copied name and rate, for a selected customer.
    for v_row in select * from jsonb_array_elements(v_crates) loop
        if v_row ->> 'name' is null or (v_row ->> 'rate_minor') is null
            or (v_row ->> 'rate_minor')::bigint < 0
            or (v_row ->> 'quantity')::integer is null or (v_row ->> 'quantity')::integer <= 0
            or jsonb_typeof(v_row -> 'components') is distinct from 'array'
            or jsonb_array_length(v_row -> 'components') = 0 then
            raise exception 'invalid_terms: crate row' using errcode = 'ALM05';
        end if;
    end loop;
    if p_terms ->> 'customer_id' is null then
        raise exception 'invalid_terms: pick a customer' using errcode = 'ALM05';
    end if;
    perform 1 from public.products
        where id in (select (l ->> 'product_id')::bigint from jsonb_array_elements(p_terms -> 'lines') l
            union select (c ->> 'product_id')::bigint
                from jsonb_array_elements(v_crates) r, jsonb_array_elements(r -> 'components') c)
        order by id for update;
    -- Priced: every line's product, then every crate component's pack and whole packets.
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        v_quantity := (v_line ->> 'quantity')::integer;
        if not exists (select 1 from public.products where id = (v_line ->> 'product_id')::bigint)
            or v_quantity is null or v_quantity <= 0
            or (v_line ->> 'unit_price_minor')::bigint is null then
            raise exception 'invalid_terms' using errcode = 'ALM05';
        end if;
    end loop;
    for v_row in select * from jsonb_array_elements(v_crates) loop
        for v_comp in select * from jsonb_array_elements(v_row -> 'components') loop
            if (v_comp ->> 'piece')::boolean then
                v_pack := (select pack from public.checkout_terms
                    where product_id = (v_comp ->> 'product_id')::bigint);
            elsif exists (select 1 from public.products
                    where id = (v_comp ->> 'product_id')::bigint) then
                v_pack := 1;
            else
                v_pack := null;
            end if;
            if v_pack is null or (v_comp ->> 'quantity')::integer is null
                or (v_comp ->> 'quantity')::integer <= 0 then
                raise exception 'invalid_terms: component terms' using errcode = 'ALM05';
            end if;
            v_demand := (v_comp ->> 'quantity')::bigint * (v_row ->> 'quantity')::bigint;
            if v_demand % v_pack <> 0 then
                raise exception 'invalid_terms: whole packs' using errcode = 'ALM05';
            end if;
            v_packs := v_packs || to_jsonb(v_pack);
            v_demands := v_demands || jsonb_build_object(
                'product_id', (v_comp ->> 'product_id')::bigint, 'packets', v_demand / v_pack);
            v_units := v_units + v_demand / v_pack;
        end loop;
        v_total := v_total + (v_row ->> 'rate_minor')::bigint * (v_row ->> 'quantity')::bigint;
        v_rows := v_rows || jsonb_build_object('quantity', (v_row ->> 'quantity')::integer,
            'components', (select jsonb_agg(jsonb_build_array((c ->> 'product_id')::bigint,
                    (c ->> 'quantity')::integer, (c ->> 'piece')::boolean) order by n)
                from jsonb_array_elements(v_row -> 'components') with ordinality t(c, n)),
            'name', v_row ->> 'name', 'rate', (v_row ->> 'rate_minor')::bigint);
    end loop;
    -- At the current sell rates.
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines') loop
        select * into v_product from public.products where id = (v_line ->> 'product_id')::bigint;
        v_quantity := (v_line ->> 'quantity')::integer;
        v_price := (v_line ->> 'unit_price_minor')::bigint;
        if v_price <> v_product.sell_minor then
            raise exception 'stale_review' using errcode = 'ALM03';
        end if;
        v_total := v_total + v_price * v_quantity;
        v_units := v_units + v_quantity;
    end loop;
    v_total := v_total + v_loading * v_units;
    if not exists (select 1 from public.customers
            where id = p_terms ->> 'customer_id' and name = p_terms ->> 'customer') then
        raise exception 'invalid_terms: pick a customer' using errcode = 'ALM05';
    end if;
    if alm_private.text_date(p_terms -> 'date') is null then
        raise exception 'invalid_terms: bill date' using errcode = 'ALM05';
    end if;
    if v_paid < 0 or v_paid > v_total then
        raise exception 'invalid_terms: amount paid' using errcode = 'ALM05';
    end if;
    -- Every demand, lines then components, against the stock left by the earlier ones.
    if exists (select 1 from (
                select product_id, sum(q) as d from (
                    select (l ->> 'product_id')::bigint as product_id,
                        (l ->> 'quantity')::bigint as q
                    from jsonb_array_elements(p_terms -> 'lines') l
                    union all
                    select (d ->> 'product_id')::bigint, (d ->> 'packets')::bigint
                    from jsonb_array_elements(v_demands) d) x
                group by product_id) n
            join public.products p on p.id = n.product_id where p.stock < n.d) then
        raise exception 'insufficient_stock' using errcode = 'ALM04';
    end if;
    v_rev := alm_private.bump();
    update public.invoice_counters set next_no = next_no + 1
        where series = 'sale' returning next_no - 1 into v_invoice;
    insert into public.sales (invoice_no, draft_id, total_minor, customer_id, customer_name,
            sale_date, paid_minor, loading_minor, rev, epoch, crate_rows, crate_packs)
        values (v_invoice, v_draft, v_total, p_terms ->> 'customer_id', p_terms ->> 'customer',
            alm_private.text_date(p_terms -> 'date'), v_paid, v_loading, v_rev, alm_private.move(),
            v_rows, v_packs)
        returning id into v_sale;
    for v_line in select * from jsonb_array_elements(p_terms -> 'lines')
        union all select * from jsonb_array_elements(v_demands)
    loop
        if v_line ? 'packets' then
            v_quantity := (v_line ->> 'packets')::integer;
            v_price := 0;
        else
            v_quantity := (v_line ->> 'quantity')::integer;
            v_price := (v_line ->> 'unit_price_minor')::bigint;
        end if;
        insert into public.sale_lines (sale_id, position, product_id, quantity, unit_price_minor,
                amount_minor, item)
            values (v_sale, v_position, (v_line ->> 'product_id')::bigint, v_quantity, v_price,
                v_price * v_quantity,
                (select item from public.products where id = (v_line ->> 'product_id')::bigint));
        v_line_cost := alm_private.allocate(v_sale, v_position, (v_line ->> 'product_id')::bigint,
            v_quantity);
        update public.sale_lines set cost_minor = v_line_cost
            where sale_id = v_sale and position = v_position;
        update public.products set rev = v_rev where id = (v_line ->> 'product_id')::bigint;
        if v_line_cost is null then
            v_known := false;
        else
            v_cost := v_cost + v_line_cost;
        end if;
        v_position := v_position + 1;
    end loop;
    update public.sales set cost_minor = case when v_known then v_cost end where id = v_sale;
    v_receipt := jsonb_build_object(
        'draft_id', v_draft, 'invoice', v_invoice, 'quantity', v_units, 'total_minor', v_total);
    insert into public.sale_requests (request_id, terms, sale_id, receipt)
        values (p_request_id, p_terms, v_sale, v_receipt);
    return v_receipt;
end;
$$;

create or replace function public.sell_crate(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_draft text := p_terms ->> 'draft_id';
    v_template public.crate_templates%rowtype;
    v_crates integer := (p_terms ->> 'quantity')::integer;
    v_paid bigint := (p_terms ->> 'paid_minor')::bigint;
    v_components jsonb;
    v_c record;
    v_pack integer;
    v_packs jsonb := '[]';
    v_demands jsonb := '[]';
    v_demand bigint;
    v_packets bigint := 0;
    v_loading bigint := (select loading_minor from public.authority_state where id = 1);
    v_total bigint;
    v_invoice bigint;
    v_sale bigint;
    v_rev bigint;
    v_line_cost bigint;
    v_cost bigint := 0;
    v_known boolean := true;
    v_position integer := 0;
    v_receipt jsonb;
begin
    perform alm_private.require_owner();
    if p_request_id is null or v_draft is null or v_crates is null or v_crates <= 0
        or v_paid is null or p_terms -> 'rate_minor' is null then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    if exists (select 1 from public.sale_requests where request_id = p_request_id) then
        raise exception 'request_conflict' using errcode = 'ALM01';
    end if;
    v_done := alm_private.replay(p_request_id, 'sell_crate', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    select * into v_template from public.crate_templates
        where id = (p_terms ->> 'template')::bigint;
    if not found then
        raise exception 'not_found: crate' using errcode = 'ALM07';
    end if;
    v_components := coalesce((select jsonb_agg(jsonb_build_object('product_id', product_id,
            'quantity', quantity, 'piece', piece) order by position)
        from public.crate_components where crate_id = v_template.id), '[]');
    if v_template.price_minor <> (p_terms ->> 'rate_minor')::bigint
        or v_components <> coalesce(p_terms -> 'components', '[]') then
        raise exception 'stale_review' using errcode = 'ALM03';
    end if;
    perform alm_private.xact_lock(2, 'sale_draft:' || v_draft);
    if exists (select 1 from public.sales where draft_id = v_draft) then
        raise exception 'draft_already_posted' using errcode = 'ALM02';
    end if;
    if p_terms ->> 'customer_id' is null or not exists (select 1 from public.customers
            where id = p_terms ->> 'customer_id' and name = p_terms ->> 'customer') then
        raise exception 'invalid_terms: pick a customer' using errcode = 'ALM05';
    end if;
    if alm_private.text_date(p_terms -> 'date') is null then
        raise exception 'invalid_terms: bill date' using errcode = 'ALM05';
    end if;
    for v_c in select * from public.crate_components where crate_id = v_template.id
        order by position
    loop
        if v_c.piece then
            v_pack := (select pack from public.checkout_terms where product_id = v_c.product_id);
        elsif exists (select 1 from public.products where id = v_c.product_id) then
            v_pack := 1;
        else
            v_pack := null;
        end if;
        if v_pack is null then
            raise exception 'invalid_terms: component terms' using errcode = 'ALM05';
        end if;
        v_packs := v_packs || to_jsonb(v_pack);
    end loop;
    v_total := v_template.price_minor * v_crates;
    for v_c in select c.*, (v_packs ->> (c.position))::integer as pack
        from public.crate_components c where c.crate_id = v_template.id order by c.position
    loop
        v_demand := v_c.quantity::bigint * v_crates;
        if v_demand % v_c.pack <> 0 then
            raise exception 'invalid_terms: whole packs' using errcode = 'ALM05';
        end if;
        v_demands := v_demands || to_jsonb(v_demand / v_c.pack);
        v_packets := v_packets + v_demand / v_c.pack;
    end loop;
    v_total := v_total + v_loading * v_packets;
    if v_paid < 0 or v_paid > v_total then
        raise exception 'invalid_terms: amount paid' using errcode = 'ALM05';
    end if;
    -- Every demand, in component order, against the stock left by the earlier ones.
    perform 1 from public.products
        where id in (select product_id from public.crate_components where crate_id = v_template.id)
        order by id for update;
    if exists (select 1 from (select c.product_id, sum((v_demands ->> c.position)::bigint) as d
                from public.crate_components c where c.crate_id = v_template.id
                group by c.product_id) n
            join public.products p on p.id = n.product_id where p.stock < n.d) then
        raise exception 'insufficient_stock' using errcode = 'ALM04';
    end if;
    v_rev := alm_private.bump();
    update public.invoice_counters set next_no = next_no + 1
        where series = 'sale' returning next_no - 1 into v_invoice;
    insert into public.sales (invoice_no, draft_id, total_minor, customer_id, customer_name,
            sale_date, paid_minor, loading_minor, rev, epoch, crate_row, crate_packs,
            checkout_loading)
        values (v_invoice, v_draft, v_total, p_terms ->> 'customer_id', p_terms ->> 'customer',
            alm_private.text_date(p_terms -> 'date'), v_paid, 0, v_rev, alm_private.move(),
            jsonb_build_object('quantity', v_crates,
                'components', (select jsonb_agg(jsonb_build_array(product_id, quantity, piece)
                    order by position) from public.crate_components
                    where crate_id = v_template.id),
                'name', v_template.name, 'rate', v_template.price_minor),
            v_packs, v_loading)
        returning id into v_sale;
    for v_c in select * from public.crate_components where crate_id = v_template.id
        order by position
    loop
        v_demand := (v_demands ->> v_position)::bigint;
        insert into public.sale_lines (sale_id, position, product_id, quantity, unit_price_minor,
                amount_minor, item)
            values (v_sale, v_position, v_c.product_id, v_demand, 0, 0,
                (select item from public.products where id = v_c.product_id));
        v_line_cost := alm_private.allocate(v_sale, v_position, v_c.product_id,
            v_demand::integer);
        update public.sale_lines set cost_minor = v_line_cost
            where sale_id = v_sale and position = v_position;
        update public.products set rev = v_rev where id = v_c.product_id;
        if v_line_cost is null then
            v_known := false;
        else
            v_cost := v_cost + v_line_cost;
        end if;
        v_position := v_position + 1;
    end loop;
    update public.sales set cost_minor = case when v_known then v_cost end where id = v_sale;
    perform alm_private.after_bill(v_draft);
    v_receipt := jsonb_build_object(
        'draft_id', v_draft, 'invoice', v_invoice, 'quantity', v_packets, 'total_minor', v_total);
    return alm_private.record(p_request_id, 'sell_crate', p_terms, v_receipt);
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.sell_crate(text, jsonb) from public, anon;
grant execute on function public.sell_crate(text, jsonb) to authenticated, service_role;
