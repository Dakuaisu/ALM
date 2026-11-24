-- Local development schema, continued: the Settings default reorder level is an authority
-- setting. `set_reorder_default` stores it; a product saved with a blank reorder level takes it
-- (the core's `form.reorder_level.unwrap_or(product_reorder_default)`, for a new product and an
-- edit alike). Synthetic slice only (OPEN-002); Owner only (OPEN-004).

-- {"level": n}
create function public.set_reorder_default(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_level integer;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'set_reorder_default', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if jsonb_typeof(p_terms -> 'level') is distinct from 'number'
        or (p_terms ->> 'level')::numeric < 0 or (p_terms ->> 'level')::numeric > 2147483647
        or (p_terms ->> 'level')::numeric <> trunc((p_terms ->> 'level')::numeric) then
        perform alm_private.invalid('reorder level');
    end if;
    v_level := (p_terms ->> 'level')::integer;
    perform alm_private.bump();
    update public.authority_state set reorder_default = v_level where id = 1;
    return alm_private.record(p_request_id, 'set_reorder_default', p_terms, '{"done": true}');
end;
$$;

create or replace function public.save_product(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_id bigint := (p_terms ->> 'id')::bigint;
    v_buy bigint := (select buy_minor from public.products where id = v_id);
    v_before bigint := alm_private.revision();
    v_out jsonb := alm_private.save_product_body(p_request_id, p_terms);
begin
    if alm_private.revision() > v_before then
        if (p_terms ->> 'new')::boolean then
            insert into public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                values (v_id, null, 0, null);
            update public.products set purchase_item = item, purchase_buy_minor = buy_minor
                where id = v_id;
        elsif v_buy is distinct from (p_terms ->> 'buy_minor')::bigint then
            update public.products set purchase_buy_minor = buy_minor where id = v_id;
        end if;
        if p_terms ->> 'reorder' is null then
            update public.products set reorder_level =
                    (select reorder_default from public.authority_state where id = 1)
                where id = v_id;
        end if;
    end if;
    return v_out;
end;
$$;

revoke all on function public.set_reorder_default(text, jsonb) from public, anon;
grant execute on function public.set_reorder_default(text, jsonb) to authenticated, service_role;
