-- Local development schema, continued: a Manager's projection carries the customers its New
-- Bill may select, as names only (`customer_names`: id and name; no balance, mobile or
-- address), in the Customers list's order. The Owner's rows are unchanged. The body is
-- 20261128050000's pull_projection_body with that one list added for the Manager.

create or replace function alm_private.pull_projection_body(p_checkpoint bigint)
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
    v_balances jsonb;
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
    -- The catalog a draft is made from: directory products at their provisional rate.
    v_rows := jsonb_build_object('catalog', coalesce((select jsonb_agg(jsonb_build_object(
            'id', id, 'name', item, 'rate', provisional_minor) order by id)
        from public.products where provisional_minor is not null and rev > v_since), '[]'));
    if v_role = 'manager' then
        -- The customers a Manager's New Bill may select: names only, no balance or contact.
        v_rows := v_rows || jsonb_build_object('customer_names', coalesce((select jsonb_agg(
                jsonb_build_object('id', c.id, 'name', c.name) order by c.name)
            from public.customers c), '[]'));
    end if;
    if v_role = 'owner' then
        -- Each customer's balance once, not once per bill.
        v_balances := coalesce((select jsonb_object_agg(c.id, alm_private.customer_balance(c.id))
            from public.customers c), '{}');
        v_rows := v_rows || jsonb_build_object(
            'sales_unsupported', null,
            'products', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'item', item,
                    'brand', brand, 'sell', sell_minor, 'reorder_level', reorder_level)
                    order by ord)
                from public.products where rev > v_since), '[]'),
            'stock', coalesce((select jsonb_agg(jsonb_build_object('product', id, 'item', item,
                    'brand', brand, 'on_hand', stock, 'reorder_level', reorder_level)
                    order by stock, ord)
                from public.products where rev > v_since), '[]'),
            'customers', coalesce((select jsonb_agg(jsonb_build_object('id', c.id, 'name', c.name,
                    'balance', (v_balances ->> c.id)::bigint) order by c.name)
                from public.customers c), '[]'),
            'sales', coalesce((select jsonb_agg(jsonb_build_object('draft_id', draft_id,
                    'invoice', invoice_no, 'date', sale_date, 'customer', customer_name,
                    'items', (select coalesce(jsonb_agg(item order by position), '[]')
                        from public.sale_lines l where l.sale_id = s.id),
                    'lines', (select count(*) from public.sale_lines l where l.sale_id = s.id),
                    'packets', (select coalesce(sum(quantity), 0) from public.sale_lines l
                        where l.sale_id = s.id),
                    'total', total_minor, 'paid', paid_minor, 'online', false,
                    'cancelled', cancelled,
                    'balance', case when cancelled then 0
                        when customer_id is null then total_minor - paid_minor
                        else coalesce((v_balances ->> customer_id)::bigint,
                            alm_private.customer_balance(customer_id)) end)
                    order by id desc)
                from public.sales s where rev > v_since), '[]'),
            'receipts', coalesce((select jsonb_agg(jsonb_build_object('slot', slot,
                    'date', receipt_date, 'customer', name, 'amount', amount_minor,
                    'online', online) order by slot)
                from (select r.*, c.name, row_number() over (order by r.id) - 1 as slot
                    from public.receipts r join public.customers c on c.id = r.customer_id) r
                where rev > v_since), '[]'),
            'cash_days', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'date', day,
                    'cash', cash_minor, 'paytm', paytm_minor, 'phonepe', phonepe_minor,
                    'goods', goods_minor, 'misc', misc_minor, 'salary', salary_minor,
                    'debt', debt_minor) order by day desc, id desc)
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
                        order by ord) from public.products where rev > v_since), '[]'),
                'supplier_prices', coalesce((select jsonb_agg(jsonb_build_array(q.product_id,
                        q.supplier_id, q.rate_minor) order by p.ord, q.supplier_id)
                    from public.supplier_prices q join public.products p on p.id = q.product_id
                    where q.rev > v_since), '[]'),
                'purchase_orders', coalesce((select jsonb_agg(jsonb_build_object('id', o.id,
                        'supplier', o.supplier_name, 'date', o.order_date,
                        'status', case
                            when not exists (select 1 from public.purchase_lines l
                                where l.po_id = o.id and l.ordered - l.received - l.closed > 0)
                                then 'Completed'
                            when exists (select 1 from public.purchase_lines l
                                where l.po_id = o.id and (l.received > 0 or l.closed > 0))
                                then 'PartiallyReceived'
                            else 'Pending' end,
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
                        s.cost_minor) order by s.id desc)
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
