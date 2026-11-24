-- Local development schema, continued: crate sales as an authority command, following the core
-- (`alm_core::confirmation::owner_billing::sell_crate`, checked by the differential suite).
-- Synthetic slice only; not the deployed schema (OPEN-002) nor a permission matrix (OPEN-004):
-- the Owner may sell a crate, a Manager may not.
--
-- {"draft_id": text, "template", "quantity", "customer_id", "customer", "date", "paid_minor",
--  "rate_minor", "components": [{"product_id", "quantity", "piece"}]}: `quantity` crates of a
-- saved template, with its rate and components as the Owner reviewed them. The bill's only row
-- is a copy of the template; each component's packets (quantity x crates, divided by the pack
-- for piece components) are taken from that product's FIFO lots as a separate demand, so a
-- product repeated in a template takes from its lots once per component (the L10 / OPEN-010
-- behavior the core implements: the cost is that of the depleted lots). The total is the rate
-- times the crates plus the authority's loading charge per packet. Returns the sale receipt.
--
-- Rejections, in the core's order: unknown template (ALM07); rate or components changed since
-- review (ALM03); draft already posted (ALM02); not the selected directory customer, a piece
-- component without checkout terms, a component not dividing into whole packs, paid outside
-- 0..total (ALM05); a shortage (ALM04).

alter table public.sales
    add column crate_row jsonb,
    add column crate_packs jsonb,
    add column checkout_loading bigint;

create function public.sell_crate(p_request_id text, p_terms jsonb)
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
    perform pg_advisory_xact_lock(hashtextextended('sale_draft:' || v_draft, 0));
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

-- A crate bill cannot be edited, cancelled or restored (the core declines it).
create or replace function public.set_sale_cancelled(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_was boolean;
    v_out jsonb;
begin
    perform alm_private.require_owner();
    v_out := alm_private.replay(p_request_id, 'set_sale_cancelled', p_terms);
    if v_out is not null then
        return v_out;
    end if;
    if exists (select 1 from public.sales where draft_id = p_terms ->> 'draft_id'
            and crate_row is not null) then
        raise exception 'invalid_terms: crate bill' using errcode = 'ALM05';
    end if;
    v_was := (select cancelled from public.sales where draft_id = p_terms ->> 'draft_id');
    v_out := alm_private.set_sale_cancelled_body(p_request_id, p_terms);
    update public.sales set edits = edits + 1
        where draft_id = p_terms ->> 'draft_id' and cancelled is distinct from v_was;
    return v_out;
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
    if exists (select 1 from public.sales where draft_id = p_terms ->> 'draft_id'
            and crate_row is not null)
        and alm_private.replay(p_request_id, 'edit_sale', p_terms) is null then
        raise exception 'invalid_terms: crate bill' using errcode = 'ALM05';
    end if;
    v_out := alm_private.edit_sale_body(p_request_id, p_terms);
    if alm_private.revision() > v_before then
        update public.sales set edits = edits + 1 where draft_id = p_terms ->> 'draft_id';
        perform alm_private.after_bill(p_terms ->> 'draft_id');
    end if;
    return v_out;
end;
$$;

-- Crate bills in the projection rows. The Sales list cannot represent them (the core's L04
-- boundary): with any crate bill it is replaced by `sales_unsupported`, the newest crate bill,
-- and the sale costs it carries are dropped with it. The book carries the crate row, checkout
-- terms and packs, with the template rate as the bill's price.
create function alm_private.crate_rows(p_out jsonb)
returns jsonb
language plpgsql
stable
set search_path = ''
as $$
declare
    v_sale record;
    v_rows jsonb := p_out -> 'rows';
    v_newest text := (select draft_id from public.sales where crate_row is not null
        order by id desc limit 1);
begin
    if v_newest is null then
        return p_out;
    end if;
    v_rows := v_rows || jsonb_build_object('sales', '[]'::jsonb,
        'sales_unsupported', v_newest::bigint);
    v_rows := jsonb_set(v_rows, '{owner,sale_costs}', '[]');
    for v_sale in select * from public.sales where crate_row is not null loop
        v_rows := jsonb_set(v_rows, '{owner,book,sales}', coalesce((select jsonb_agg(case
                when e ->> 'draft' = v_sale.draft_id then e || jsonb_build_object(
                    'lines', '[]'::jsonb,
                    'prices', jsonb_build_array((v_sale.crate_row ->> 'rate')::bigint),
                    'crate_row', v_sale.crate_row,
                    'checkout', jsonb_build_object('lines', '[]'::jsonb,
                        'loading', v_sale.checkout_loading),
                    'crate_packs', v_sale.crate_packs)
                else e end order by n)
            from jsonb_array_elements(v_rows #> '{owner,book,sales}') with ordinality t(e, n)),
            '[]'));
    end loop;
    return jsonb_set(p_out, '{rows}', v_rows);
end;
$$;

create or replace function public.pull_projection(p_checkpoint bigint)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
declare
    v_out jsonb := alm_private.pull_projection_body(p_checkpoint);
begin
    if v_out ->> 'role' = 'owner' and jsonb_typeof(v_out -> 'rows') = 'object' then
        v_out := jsonb_set(v_out, '{rows,owner,book}', alm_private.owner_book());
        v_out := alm_private.crate_rows(v_out);
    end if;
    return v_out;
end;
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.sell_crate(text, jsonb) from public, anon;
grant execute on function public.sell_crate(text, jsonb) to authenticated, service_role;
