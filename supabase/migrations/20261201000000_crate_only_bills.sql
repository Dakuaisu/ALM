-- Local development schema, continued: a bill of crate rows only (several crate rows and no item
-- line) posts through `post_sale`, as the HTML's `saveBill` accepts a bill whose lines are all
-- crates and as the core does. Before, `post_sale_body` refused an empty `lines` array beside
-- crate rows ('invalid_terms'). A bill with neither lines nor crate rows still goes to
-- `post_sale_lines_body`, which refuses it. The body is otherwise unchanged; the rewrite must
-- match exactly once.

do $migration$
declare
    v_old text := $old$        or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(p_terms -> 'lines') = 0 or v_loading < 0 then$old$;
    v_new text := $new$        or jsonb_typeof(p_terms -> 'lines') <> 'array'
        or jsonb_array_length(v_crates) = 0 or v_loading < 0 then$new$;
    v_source text := pg_get_functiondef('alm_private.post_sale_body(text, jsonb)'::regprocedure);
begin
    if (length(v_source) - length(replace(v_source, v_old, ''))) / length(v_old) <> 1 then
        raise exception 'crate_only_bills: post_sale_body does not hold the expected text once';
    end if;
    execute replace(v_source, v_old, v_new);
end
$migration$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
