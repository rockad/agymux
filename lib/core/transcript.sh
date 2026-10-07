#!/usr/bin/env bash
# transcript.sh - Fast jq-based transcript parser & preview generator
# Part of agymux v0.2 core engine.
# Designed for ultra-fast execution (<15ms) for live fzf preview panes.

set -euo pipefail

BRAIN_DIR="${AGYMUX_BRAIN_DIR:-${HOME}/.gemini/antigravity-cli/brain}"

# Locate transcript JSONL file for a given conversation_id
find_transcript_path() {
    local cid="${1:-}"
    [[ -z "$cid" ]] && return 1

    local path="${BRAIN_DIR}/${cid}/.system_generated/logs/transcript.jsonl"
    if [[ -f "$path" ]]; then
        echo "$path"
        return 0
    fi

    # Fallback to transcript_full.jsonl if present
    local full_path="${BRAIN_DIR}/${cid}/.system_generated/logs/transcript_full.jsonl"
    if [[ -f "$full_path" ]]; then
        echo "$full_path"
        return 0
    fi

    return 1
}

# Parse transcript JSONL using jq into a structured JSON object
parse_transcript_json() {
    local cid="${1:-}"
    local jsonl_path
    jsonl_path=$(find_transcript_path "$cid") || {
        echo "{\"error\": \"transcript_not_found\", \"conversation_id\": \"$cid\"}"
        return 1
    }

    jq -n --arg cid "$cid" '
      def clean_prompt:
        if . == null then ""
        else
          gsub("(?s)<USER_REQUEST>\\s*|\\s*</USER_REQUEST>"; "")
          | gsub("(?s)<ADDITIONAL_METADATA>.*?</ADDITIONAL_METADATA>"; "")
          | gsub("(?s)<USER_SETTINGS_CHANGE>.*?</USER_SETTINGS_CHANGE>"; "")
          | gsub("^\\s+|\\s+$"; "")
        end;

      reduce inputs as $item (
        {
          conversation_id: $cid,
          first_prompt: null,
          latest_turn: null,
          tools: {},
          total_tools: 0,
          tokens: { input: 0, output: 0, cache: 0, total: 0 }
        };
        (if .first_prompt == null and ($item.type == "USER_INPUT" or $item.source == "USER_EXPLICIT") then
            .first_prompt = ($item.content // "" | clean_prompt)
         else . end)
        | (if $item.type == "PLANNER_RESPONSE" and (($item.content // "") != "" or ($item.thinking // "") != "") then
            .latest_turn = (if ($item.content // "") != "" then $item.content else $item.thinking end)
           else . end)
        | (if $item.input_tokens then .tokens.input += $item.input_tokens else . end)
        | (if $item.output_tokens then .tokens.output += $item.output_tokens else . end)
        | (if $item.cache_read_tokens then .tokens.cache += $item.cache_read_tokens else . end)
        | (if $item.tool_calls then
             reduce $item.tool_calls[] as $t (.;
               .total_tools += 1 |
               .tools[$t.name] = ((.tools[$t.name] // 0) + 1)
             )
           else . end)
      ) |
      .tokens.total = (.tokens.input + .tokens.output + .tokens.cache) |
      .
    ' "$jsonl_path"
}

# Format numbers with K/M suffixes or commas
format_number() {
    local num="${1:-0}"
    if (( num >= 1000000 )); then
        awk -v n="$num" 'BEGIN { printf "%.1fM", n/1000000 }'
    elif (( num >= 1000 )); then
        awk -v n="$num" 'BEGIN { printf "%.1fk", n/1000 }'
    else
        echo "$num"
    fi
}

# Format rich human-readable preview for fzf preview pane
format_transcript_preview() {
    local cid="${1:-}"
    local jsonl_path
    jsonl_path=$(find_transcript_path "$cid") || {
        echo -e "\033[1;31m[!] Transcript not found for conversation:\033[0m $cid"
        return 0
    }

    # Extract all metrics in a single jq pass directly into formatted text
    jq -n -r --arg cid "$cid" '
      def clean_prompt:
        if . == null then ""
        else
          gsub("(?s)<USER_REQUEST>\\s*|\\s*</USER_REQUEST>"; "")
          | gsub("(?s)<ADDITIONAL_METADATA>.*?</ADDITIONAL_METADATA>"; "")
          | gsub("(?s)<USER_SETTINGS_CHANGE>.*?</USER_SETTINGS_CHANGE>"; "")
          | gsub("^\\s+|\\s+$"; "")
        end;

      def truncate_str(len):
        if . == null then ""
        elif (length > len) then (.[0:len] + " ...")
        else . end;

      def fmt_num:
        if . >= 1000000 then ((. / 100000 | round / 10 | tostring) + "M")
        elif . >= 1000 then ((. / 100 | round / 10 | tostring) + "k")
        else tostring end;

      reduce inputs as $item (
        {
          first_prompt: null,
          latest_turn: null,
          tools: {},
          total_tools: 0,
          input: 0,
          output: 0,
          cache: 0
        };
        (if .first_prompt == null and ($item.type == "USER_INPUT" or $item.source == "USER_EXPLICIT") then
            .first_prompt = ($item.content // "" | clean_prompt)
         else . end)
        | (if $item.type == "PLANNER_RESPONSE" and (($item.content // "") != "" or ($item.thinking // "") != "") then
            .latest_turn = (if ($item.content // "") != "" then $item.content else $item.thinking end)
           else . end)
        | (if $item.input_tokens then .input += $item.input_tokens else . end)
        | (if $item.output_tokens then .output += $item.output_tokens else . end)
        | (if $item.cache_read_tokens then .cache += $item.cache_read_tokens else . end)
        | (if $item.tool_calls then
             reduce $item.tool_calls[] as $t (.;
               .total_tools += 1 |
               .tools[$t.name] = ((.tools[$t.name] // 0) + 1)
             )
           else . end)
      ) |
      (
        "\u001b[1;36m━━━ Conversation Details ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\u001b[0m\n" +
        "\u001b[1;33mID:\u001b[0m     " + $cid + "\n" +
        "\u001b[1;33mTokens:\u001b[0m Input: \u001b[1;32m" + (.input | fmt_num) + "\u001b[0m | Output: \u001b[1;32m" + (.output | fmt_num) + "\u001b[0m | Cache: \u001b[1;32m" + (.cache | fmt_num) + "\u001b[0m (Total: " + ((.input + .output + .cache) | fmt_num) + ")\n" +
        "\u001b[1;33mTools:\u001b[0m  " + (.total_tools | tostring) + " executed\n\n" +
        "\u001b[1;36m━━━ Initial User Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\u001b[0m\n" +
        (if .first_prompt != null and .first_prompt != "" then
           (.first_prompt | truncate_str(350))
         else "\u001b[0;90m(No initial prompt recorded)\u001b[0m" end) + "\n\n" +
        "\u001b[1;36m━━━ Latest Assistant Turn ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\u001b[0m\n" +
        (if .latest_turn != null and .latest_turn != "" then
           (.latest_turn | truncate_str(450))
         else "\u001b[0;90m(No assistant responses yet)\u001b[0m" end) + "\n\n" +
        "\u001b[1;36m━━━ Executed Tools Breakdown ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\u001b[0m\n" +
        (if .total_tools > 0 then
           ([.tools | to_entries | sort_by(-.value)[] | "  \u001b[0;32m●\u001b[0m \u001b[1m\(.key)\u001b[0m: \u001b[0;33m\(.value)\u001b[0m"] | join("\n"))
         else "  \u001b[0;90mNone\u001b[0m" end)
      )
    ' "$jsonl_path"
}

# CLI entry point if run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    subcmd="${1:-}"
    case "$subcmd" in
        json|--json)
            parse_transcript_json "${2:-}"
            ;;
        path)
            find_transcript_path "${2:-}"
            ;;
        preview|--preview|"")
            if [[ "$subcmd" == "preview" || "$subcmd" == "--preview" ]]; then
                conv="${2:-}"
            else
                conv="$subcmd"
            fi
            [[ -z "$conv" ]] && { echo "Usage: $0 [preview|json|path] <conversation_id>" >&2; exit 1; }
            format_transcript_preview "$conv"
            ;;
        *)
            format_transcript_preview "$subcmd"
            ;;
    esac
fi
