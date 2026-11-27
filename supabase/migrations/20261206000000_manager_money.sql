-- Local development schema, continued: customers and money for a Manager (DEC-034).
-- `save_customer` (Owner and Manager) adds a customer, numbered `cust-<n>` by the authority,
-- or changes a customer's mobile and address, as the HTML's `editCustomer` saves them (fields
-- trimmed, a name required and not another customer's, case-insensitive); the opening balance
-- stays the Owner's `save_opening`. A Manager's pull also carries `customers` with their
-- balances, mobiles and addresses, as the Owner's does. Recording payments and closing today's
-- Cash Book were opened to a Manager by the matrix (20261204000000).

insert into alm_private.permissions (function_name, manager) values ('save_customer', true);

alter table public.authority_state add column next_customer bigint not null default 1
    check (next_customer >= 1);

create function alm_private.save_customer_api(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id text := nullif(p_terms ->> 'id', '');
    v_name text := btrim(coalesce(p_terms ->> 'name', ''));
    v_mobile text := btrim(coalesce(p_terms ->> 'mobile', ''));
    v_addr1 text := btrim(coalesce(p_terms ->> 'addr1', ''));
    v_addr2 text := btrim(coalesce(p_terms ->> 'addr2', ''));
    v_twin text;
    v_n bigint;
    v_rev bigint;
begin
    v_done := alm_private.replay(p_request_id, 'save_customer', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if v_id is not null then
        if not exists (select 1 from public.customers where id = v_id and registered) then
            raise exception 'not_found: customer' using errcode = 'ALM07';
        end if;
        v_rev := alm_private.bump();
        update public.customers set mobile = v_mobile, addr1 = v_addr1, addr2 = v_addr2,
                rev = v_rev
            where id = v_id;
        return alm_private.record(p_request_id, 'save_customer', p_terms, '{"done": true}');
    end if;
    if v_name = '' then
        raise exception 'invalid_terms: Enter name' using errcode = 'ALM05';
    end if;
    select name into v_twin from public.customers
        where registered and lower(btrim(name)) = lower(v_name) order by name, id limit 1;
    if found then
        raise exception 'invalid_terms: A customer called “%” already exists — add a surname or area to tell them apart',
            v_twin using errcode = 'ALM05';
    end if;
    update public.authority_state set next_customer = next_customer + 1 where id = 1
        returning next_customer - 1 into v_n;
    v_rev := alm_private.bump();
    insert into public.customers (id, name, mobile, addr1, addr2, rev, registered)
        values ('cust-' || v_n, v_name, v_mobile, v_addr1, v_addr2, v_rev, true);
    return alm_private.record(p_request_id, 'save_customer', p_terms,
        jsonb_build_object('created', v_n));
end;
$$;

create function public.save_customer(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$ begin perform alm_private.require_permission('save_customer'); perform alm_private.manager_limits('save_customer', p_request_id, p_terms); return alm_private.save_customer_api(p_request_id, p_terms); end; $$;

revoke all on function alm_private.save_customer_api(text, jsonb) from public, anon, authenticated;
revoke all on function public.save_customer(text, jsonb) from public, anon;
grant execute on function public.save_customer(text, jsonb) to authenticated, service_role;

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
        -- The customers with their balances, as the Owner's rows list them.
        v_out := jsonb_set(v_out, '{rows,customers}', coalesce((select jsonb_agg(
                jsonb_build_object('id', c.id, 'name', c.name,
                    'balance', alm_private.customer_balance(c.id)) order by c.name)
            from public.customers c), '[]'));
        v_out := alm_private.customer_contacts(v_out);
    end if;
    return v_out;
end;
$$;

revoke all on function alm_private.pull_projection_api(bigint) from public, anon, authenticated;
