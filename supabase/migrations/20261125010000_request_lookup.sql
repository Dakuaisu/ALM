-- Local development schema, continued: resolving an intent whose outcome is unknown. The
-- device sends the request ID with the command and terms it stored before sending; the answer
-- is the committed outcome, "absent" (nothing committed: retrying with the same ID is safe) or
-- "conflict" (the ID was used for another intent). No effect. Owner only, like request_status
-- (OPEN-004: who may check status).
create function public.request_lookup(p_request_id text, p_command text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
declare
    v_command text;
    v_terms jsonb;
    v_outcome jsonb;
begin
    perform alm_private.require_owner();
    if p_request_id is null or p_command is null or p_terms is null then
        raise exception 'invalid_terms' using errcode = 'ALM05';
    end if;
    select command, terms, outcome into v_command, v_terms, v_outcome
        from public.command_requests where request_id = p_request_id;
    if not found then
        select 'post_sale', terms, receipt into v_command, v_terms, v_outcome
            from public.sale_requests where request_id = p_request_id;
    end if;
    if v_command is null then
        return jsonb_build_object('state', 'absent');
    end if;
    if v_command = p_command and v_terms = p_terms then
        return jsonb_build_object('state', 'committed', 'outcome', v_outcome);
    end if;
    return jsonb_build_object('state', 'conflict');
end;
$$;

revoke all on function public.request_lookup(text, text, jsonb) from public, anon, authenticated;
grant execute on function public.request_lookup(text, text, jsonb) to authenticated, service_role;
