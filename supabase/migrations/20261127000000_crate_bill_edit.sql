-- Local development schema, continued: crate bills in Sales (OPEN-013), following the core
-- (`ConfirmationFixture::edit_crate_bill`, `cancel_sale` on a named crate bill and the Sales
-- list's crate line; checked by the differential suite). Synthetic slice only (OPEN-002).
--
-- A crate bill is listed like any bill: one item named by its crate, its component packets,
-- and its sale cost (revenue: rate x crates). It is cancelled and restored like any bill (the
-- same part-paid named-bill rule). `edit_sale` on a crate bill takes one entry, `{"quantity":
-- crates, "rate_minor": rate}`: each component's packets follow the new crate count from or
-- back to its lots (unit cost frozen at the first edit, as a line edit), the loading charge per
-- packet stays the posted one, and the crate row keeps its contents.

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
    v_was := (select cancelled from public.sales where draft_id = p_terms ->> 'draft_id');
    v_out := alm_private.set_sale_cancelled_body(p_request_id, p_terms);
    update public.sales set edits = edits + 1
        where draft_id = p_terms ->> 'draft_id' and cancelled is distinct from v_was;
    return v_out;
end;
$$;

create function alm_private.edit_crate_sale_body(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
    v_sale public.sales%rowtype;
    v_edit jsonb := p_terms -> 'lines' -> 0;
    v_crates integer;
    v_rate bigint;
    v_comp jsonb;
    v_position integer := 0;
    v_pack integer;
    v_demand bigint;
    v_demands jsonb := '[]';
    v_packets bigint := 0;
    v_total bigint;
    v_line record;
    v_quantity integer;
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
    if jsonb_typeof(p_terms -> 'lines') is distinct from 'array'
        or jsonb_array_length(p_terms -> 'lines') <> 1 then
        perform alm_private.invalid('lines');
    end if;
    v_crates := (v_edit ->> 'quantity')::integer;
    v_rate := (v_edit ->> 'rate_minor')::bigint;
    if v_crates is null or v_crates <= 0 or v_rate is null or v_rate < 0 then
        perform alm_private.invalid('line');
    end if;
    for v_comp in select e from jsonb_array_elements(v_sale.crate_row -> 'components')
        with ordinality t(e, n) order by n
    loop
        v_pack := (v_sale.crate_packs ->> v_position)::integer;
        v_demand := (v_comp ->> 1)::bigint * v_crates;
        if v_demand % v_pack <> 0 then
            raise exception 'invalid_terms: whole packs' using errcode = 'ALM05';
        end if;
        v_demands := v_demands || to_jsonb(v_demand / v_pack);
        v_packets := v_packets + v_demand / v_pack;
        v_position := v_position + 1;
    end loop;
    v_total := v_rate * v_crates + v_sale.checkout_loading * v_packets;
    if v_total < v_sale.paid_minor then
        raise exception 'invalid_terms: below paid' using errcode = 'ALM05';
    end if;
    if v_sale.customer_id is null then
        perform alm_private.invalid('customer');
    end if;
    v_rev := alm_private.bump();
    for v_line in select * from public.sale_lines where sale_id = v_sale.id order by position loop
        v_quantity := (v_demands ->> v_line.position)::integer;
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
        update public.sale_lines set quantity = v_quantity, frozen = true, frozen_unit = v_unit
            where sale_id = v_sale.id and position = v_line.position;
        if v_unit is null then
            v_known := false;
        else
            v_cost := v_cost + v_unit * v_quantity;
        end if;
    end loop;
    update public.sales set total_minor = v_total, rev = v_rev, epoch = alm_private.move(),
            cost_minor = case when v_known then v_cost end,
            crate_row = crate_row || jsonb_build_object('quantity', v_crates, 'rate', v_rate)
        where id = v_sale.id;
    return alm_private.record(p_request_id, 'edit_sale', p_terms,
        jsonb_build_object('draft_id', v_sale.draft_id, 'invoice', v_sale.invoice_no,
            'quantity', v_packets, 'total_minor', v_total));
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

-- Crate bills in the projection rows: listed in Sales (one item named by the crate, its
-- component packets), with their sale cost; the book carries the crate row, checkout terms and
-- packs, with the crate rate as the bill's price.
create or replace function alm_private.crate_rows(p_out jsonb)
returns jsonb
language plpgsql
stable
set search_path = ''
as $$
declare
    v_sale record;
    v_rows jsonb := p_out -> 'rows';
begin
    for v_sale in select * from public.sales where crate_row is not null loop
        v_rows := jsonb_set(v_rows, '{sales}', coalesce((select jsonb_agg(case
                when e ->> 'draft_id' = v_sale.draft_id then e || jsonb_build_object(
                    'items', jsonb_build_array(coalesce(v_sale.crate_row ->> 'name', '')),
                    'lines', 1)
                else e end order by n)
            from jsonb_array_elements(v_rows -> 'sales') with ordinality t(e, n)), '[]'));
        v_rows := jsonb_set(v_rows, '{owner,sale_costs}', coalesce((select jsonb_agg(case
                when e ->> 0 = v_sale.draft_id then jsonb_build_array(e -> 0,
                    (v_sale.crate_row ->> 'rate')::bigint
                        * (v_sale.crate_row ->> 'quantity')::bigint, e -> 2)
                else e end order by n)
            from jsonb_array_elements(v_rows #> '{owner,sale_costs}') with ordinality t(e, n)),
            '[]'));
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

revoke all on all tables in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.set_sale_cancelled(text, jsonb) from public, anon;
grant execute on function public.set_sale_cancelled(text, jsonb) to authenticated, service_role;
revoke all on function public.edit_sale(text, jsonb) from public, anon;
grant execute on function public.edit_sale(text, jsonb) to authenticated, service_role;
