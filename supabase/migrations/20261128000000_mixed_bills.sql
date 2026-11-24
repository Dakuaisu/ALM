-- Local development schema, continued: mixed bills (OPEN-013), following the core
-- (`ConfirmationFixture` reviews with crate rows beside ordinary lines, `edit_posted_lines`,
-- the Sales list and the owner book; checked by the differential suite). Synthetic slice only
-- (OPEN-002); not a permission matrix (OPEN-004): the Owner may post one, a Manager may not.
--
-- `post_sale` takes an optional `"crates": [{"name", "rate_minor", "quantity", "components":
-- [{"product_id", "quantity", "piece"}]}]` beside its lines (the HTML's New Bill `draft.items`
-- holding `crateRow`s and `lineRow`s). The bill lists its ordinary lines first, then its crate
-- rows. Each crate row is a copy (name, rate, contents) as the Owner reviewed it; its
-- components' packets (quantity x crates, divided by the pack for piece components) are taken
-- from each product's FIFO lots as separate demands after the ordinary lines (OPEN-010: a
-- repeated product takes from its lots once per component). The total is the ordinary lines,
-- each crate's rate x crates, and the draft's loading charge per packet of all of them.
--
-- Rejections, in the core's order: already posted (ALM02); a crate row without name or rate,
-- or no selected customer (ALM05); an unknown product, a crate component without terms or not
-- dividing into whole packs (ALM05); a stale line price (ALM03); not the directory customer,
-- no bill date, paid outside 0..total (ALM05); a shortage (ALM04).
--
-- A mixed bill is listed with one item per ordinary line and one per crate (named by the crate),
-- is edited with one entry per ordinary line then one per crate row (`{"quantity": crates,
-- "rate_minor": rate}`; components follow the crate count from or back to their lots at the
-- frozen unit cost) and is cancelled and restored like any bill.

alter table public.sales add column crate_rows jsonb;

-- Components across a mixed bill's crate rows.
create function alm_private.crate_components(p_rows jsonb)
returns integer
language sql
immutable
set search_path = ''
as $$
    select coalesce(sum(jsonb_array_length(r -> 'components')), 0)::integer
        from jsonb_array_elements(coalesce(p_rows, '[]')) r;
$$;

alter function alm_private.post_sale_body(text, jsonb) rename to post_sale_lines_body;

create function alm_private.post_sale_body(p_request_id text, p_terms jsonb)
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

-- One entry per ordinary line (quantity, rate), then one per crate row (crates, rate).
create function alm_private.edit_mixed_sale_body(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
    v_sale public.sales%rowtype;
    v_ordinary integer;
    v_crates integer;
    v_edit jsonb;
    v_row jsonb;
    v_comp jsonb;
    v_index integer;
    v_quantity integer;
    v_rate bigint;
    v_pack integer;
    v_demand bigint;
    v_targets jsonb := '[]';
    v_rows jsonb := '[]';
    v_component integer := 0;
    v_total bigint := 0;
    v_units bigint := 0;
    v_line record;
    v_unit bigint;
    v_line_cost bigint;
    v_cost bigint := 0;
    v_known boolean := true;
    v_rev bigint;
begin
    select * into v_sale from public.sales where draft_id = p_terms ->> 'draft_id' for update;
    if v_sale.cancelled then
        raise exception 'invalid_state' using errcode = 'ALM08';
    end if;
    v_crates := jsonb_array_length(v_sale.crate_rows);
    v_ordinary := (select count(*) from public.sale_lines where sale_id = v_sale.id)
        - alm_private.crate_components(v_sale.crate_rows);
    if jsonb_typeof(p_terms -> 'lines') is distinct from 'array'
        or jsonb_array_length(p_terms -> 'lines') <> v_ordinary + v_crates then
        perform alm_private.invalid('lines');
    end if;
    for v_index in 0 .. v_ordinary + v_crates - 1 loop
        v_edit := p_terms -> 'lines' -> v_index;
        v_quantity := (v_edit ->> 'quantity')::integer;
        v_rate := (v_edit ->> 'rate_minor')::bigint;
        if v_quantity is null or v_quantity <= 0 or v_rate is null or v_rate < 0 then
            perform alm_private.invalid('line');
        end if;
        v_total := v_total + v_quantity * v_rate;
        if v_index < v_ordinary then
            v_units := v_units + v_quantity;
            v_targets := v_targets || to_jsonb(v_quantity);
        else
            v_row := v_sale.crate_rows -> (v_index - v_ordinary);
            for v_comp in select e from jsonb_array_elements(v_row -> 'components')
                with ordinality t(e, n) order by n
            loop
                v_pack := (v_sale.crate_packs ->> v_component)::integer;
                v_demand := (v_comp ->> 1)::bigint * v_quantity;
                if v_demand % v_pack <> 0 then
                    raise exception 'invalid_terms: whole packs' using errcode = 'ALM05';
                end if;
                v_targets := v_targets || to_jsonb(v_demand / v_pack);
                v_units := v_units + v_demand / v_pack;
                v_component := v_component + 1;
            end loop;
            v_rows := v_rows || (v_row || jsonb_build_object('quantity', v_quantity,
                'rate', v_rate));
        end if;
    end loop;
    v_total := v_total + v_sale.loading_minor * v_units;
    if v_total < v_sale.paid_minor then
        raise exception 'invalid_terms: below paid' using errcode = 'ALM05';
    end if;
    if v_sale.customer_id is null then
        perform alm_private.invalid('customer');
    end if;
    v_rev := alm_private.bump();
    for v_line in select * from public.sale_lines where sale_id = v_sale.id order by position loop
        v_quantity := (v_targets ->> v_line.position)::integer;
        v_unit := v_line.frozen_unit;
        if not v_line.frozen then
            select case when bool_and(l.unit_cost_minor is not null)
                    then sum(a.quantity * l.unit_cost_minor) end
                into v_line_cost
                from public.sale_line_lots a join public.product_lots l on l.id = a.lot_id
                where a.sale_id = v_sale.id and a.position = v_line.position;
            v_unit := (v_line_cost * 2 + v_line.quantity) / (v_line.quantity * 2);
        end if;
        if v_quantity > v_line.quantity then
            perform alm_private.allocate(v_sale.id, v_line.position, v_line.product_id,
                v_quantity - v_line.quantity);
        elsif v_quantity < v_line.quantity then
            perform alm_private.release(v_sale.id, v_line.position, v_line.product_id,
                v_line.quantity - v_quantity);
        end if;
        update public.products set rev = v_rev where id = v_line.product_id;
        if v_line.position < v_ordinary then
            v_rate := (p_terms -> 'lines' -> v_line.position ->> 'rate_minor')::bigint;
            update public.sale_lines set quantity = v_quantity, unit_price_minor = v_rate,
                    amount_minor = v_quantity * v_rate, frozen = true, frozen_unit = v_unit
                where sale_id = v_sale.id and position = v_line.position;
        else
            update public.sale_lines set quantity = v_quantity, frozen = true,
                    frozen_unit = v_unit
                where sale_id = v_sale.id and position = v_line.position;
        end if;
        if v_unit is null then
            v_known := false;
        else
            v_cost := v_cost + v_unit * v_quantity;
        end if;
    end loop;
    update public.sales set total_minor = v_total, rev = v_rev, epoch = alm_private.move(),
            cost_minor = case when v_known then v_cost end, crate_rows = v_rows
        where id = v_sale.id;
    return alm_private.record(p_request_id, 'edit_sale', p_terms,
        jsonb_build_object('draft_id', v_sale.draft_id, 'invoice', v_sale.invoice_no,
            'quantity', v_units, 'total_minor', v_total));
end;
$$;

create or replace function public.edit_sale(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_before bigint := alm_private.revision();
    v_out jsonb;
begin
    perform alm_private.require_owner();
    v_out := alm_private.replay(p_request_id, 'edit_sale', p_terms);
    if v_out is not null then
        return v_out;
    end if;
    if exists (select 1 from public.sales where draft_id = p_terms ->> 'draft_id'
            and crate_row is not null) then
        v_out := alm_private.edit_crate_sale_body(p_request_id, p_terms);
    elsif exists (select 1 from public.sales where draft_id = p_terms ->> 'draft_id'
            and crate_rows is not null) then
        v_out := alm_private.edit_mixed_sale_body(p_request_id, p_terms);
    else
        v_out := alm_private.edit_sale_body(p_request_id, p_terms);
    end if;
    if alm_private.revision() > v_before then
        update public.sales set edits = edits + 1 where draft_id = p_terms ->> 'draft_id';
        perform alm_private.after_bill(p_terms ->> 'draft_id');
    end if;
    return v_out;
end;
$$;

-- Bills with crate rows in the projection rows. The book carries `crate_rows` (one for a crate
-- bill, every row of a mixed bill) and, for a crate bill, its checkout terms. Sales lists one
-- item per ordinary line and one per crate (named by the crate); the sale cost's revenue is the
-- ordinary line amounts plus each crate's rate x crates.
create or replace function alm_private.crate_rows(p_out jsonb)
returns jsonb
language plpgsql
stable
set search_path = ''
as $$
declare
    v_sale record;
    v_rows jsonb := p_out -> 'rows';
    v_crates jsonb;
    v_ordinary integer;
    v_items jsonb;
    v_prices jsonb;
    v_revenue bigint;
    v_checkout jsonb;
begin
    v_rows := jsonb_set(v_rows, '{owner,book,sales}', coalesce((select jsonb_agg(
            (e - 'crate_row') || jsonb_build_object('crate_rows', '[]'::jsonb) order by n)
        from jsonb_array_elements(v_rows #> '{owner,book,sales}') with ordinality t(e, n)),
        '[]'));
    for v_sale in select * from public.sales where crate_row is not null or crate_rows is not null
    loop
        v_crates := coalesce(v_sale.crate_rows, jsonb_build_array(v_sale.crate_row));
        v_ordinary := (select count(*) from public.sale_lines where sale_id = v_sale.id)
            - alm_private.crate_components(v_crates);
        v_items := coalesce((select jsonb_agg(l.item order by l.position)
                from public.sale_lines l where l.sale_id = v_sale.id
                    and l.position < v_ordinary and l.item is not null), '[]')
            || coalesce((select jsonb_agg(coalesce(r ->> 'name', '') order by n)
                from jsonb_array_elements(v_crates) with ordinality t(r, n)), '[]');
        v_prices := coalesce((select jsonb_agg(l.unit_price_minor order by l.position)
                from public.sale_lines l where l.sale_id = v_sale.id
                    and l.position < v_ordinary), '[]')
            || coalesce((select jsonb_agg((r ->> 'rate')::bigint order by n)
                from jsonb_array_elements(v_crates) with ordinality t(r, n)), '[]');
        v_revenue := coalesce((select sum(l.amount_minor) from public.sale_lines l
                where l.sale_id = v_sale.id and l.position < v_ordinary), 0)
            + coalesce((select sum((r ->> 'rate')::bigint * (r ->> 'quantity')::bigint)
                from jsonb_array_elements(v_crates) r), 0);
        v_checkout := case when v_sale.crate_row is not null then jsonb_build_object(
            'lines', '[]'::jsonb, 'loading', v_sale.checkout_loading) end;
        v_rows := jsonb_set(v_rows, '{sales}', coalesce((select jsonb_agg(case
                when e ->> 'draft_id' = v_sale.draft_id then e || jsonb_build_object(
                    'items', v_items, 'lines', v_ordinary + jsonb_array_length(v_crates))
                else e end order by n)
            from jsonb_array_elements(v_rows -> 'sales') with ordinality t(e, n)), '[]'));
        v_rows := jsonb_set(v_rows, '{owner,sale_costs}', coalesce((select jsonb_agg(case
                when e ->> 0 = v_sale.draft_id then jsonb_build_array(e -> 0, v_revenue, e -> 2)
                else e end order by n)
            from jsonb_array_elements(v_rows #> '{owner,sale_costs}') with ordinality t(e, n)),
            '[]'));
        v_rows := jsonb_set(v_rows, '{owner,book,sales}', coalesce((select jsonb_agg(case
                when e ->> 'draft' = v_sale.draft_id then e || jsonb_build_object(
                    'lines', coalesce((select jsonb_agg(jsonb_build_array(l.product_id,
                            l.quantity) order by l.position) from public.sale_lines l
                        where l.sale_id = v_sale.id and l.position < v_ordinary), '[]'),
                    'prices', v_prices,
                    'crate_rows', v_crates,
                    'checkout', v_checkout,
                    'crate_packs', v_sale.crate_packs)
                else e end order by n)
            from jsonb_array_elements(v_rows #> '{owner,book,sales}') with ordinality t(e, n)),
            '[]'));
    end loop;
    return jsonb_set(p_out, '{rows}', v_rows);
end;
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.edit_sale(text, jsonb) from public, anon;
grant execute on function public.edit_sale(text, jsonb) to authenticated, service_role;
