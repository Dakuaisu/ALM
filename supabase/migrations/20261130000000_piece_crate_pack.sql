-- Local development schema, continued: a crate component sold by the piece counts the product's
-- pieces per packet from its checkout terms, else 1 (the HTML's `linePackets`:
-- `packetQty || 1`), as the core does. Before, a piece component of a product without checkout
-- terms was refused ('invalid_terms: component terms') although its template saves. An unknown
-- product is still refused, and the demand must still be whole packets.
-- `post_sale_body` (bills with crate rows) and `sell_crate` (a bill of one crate) are the only
-- functions that take a component's pack; edits reuse the packs recorded on the sale. Their
-- bodies are otherwise unchanged from 20261128040000_bounded_locks.sql; each rewrite below must
-- match exactly once.

do $migration$
declare
    v_target record;
    v_source text;
    v_new text;
begin
    for v_target in
        select * from (values
            ('alm_private.post_sale_body(text, jsonb)'::regprocedure,
             $old$                v_pack := (select pack from public.checkout_terms
                    where product_id = (v_comp ->> 'product_id')::bigint);$old$,
             $new$                v_pack := coalesce((select pack from public.checkout_terms
                    where product_id = (v_comp ->> 'product_id')::bigint),
                    (select 1 from public.products
                        where id = (v_comp ->> 'product_id')::bigint));$new$),
            ('public.sell_crate(text, jsonb)'::regprocedure,
             $old$            v_pack := (select pack from public.checkout_terms where product_id = v_c.product_id);$old$,
             $new$            v_pack := coalesce(
                (select pack from public.checkout_terms where product_id = v_c.product_id),
                (select 1 from public.products where id = v_c.product_id));$new$)
        ) as t(fn, old_text, new_text)
    loop
        v_source := pg_get_functiondef(v_target.fn);
        if (length(v_source) - length(replace(v_source, v_target.old_text, '')))
                / length(v_target.old_text) <> 1 then
            raise exception 'piece_crate_pack: % does not hold the expected text once', v_target.fn;
        end if;
        v_new := replace(v_source, v_target.old_text, v_target.new_text);
        execute v_new;
    end loop;
end
$migration$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.sell_crate(text, jsonb) from public, anon;
grant execute on function public.sell_crate(text, jsonb) to authenticated, service_role;
