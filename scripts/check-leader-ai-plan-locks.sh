#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
plan_one="$repo_root/docs/leader-ai-overhaul/final-hole-hunting-content-plan.md"
plan_two="$repo_root/docs/leader-ai-overhaul/final-integrated-overhaul-plan.md"
board_one="$repo_root/docs/leader-ai-overhaul/BOARD.md"
board_two="$repo_root/docs/branch-plan-merge/bug-gui-design-BOARD.md"
merge_board="$repo_root/docs/branch-plan-merge/BOARD.md"

expected_plan_one_hash="a21de967d2b500a76cea961f905ae90be210e2e3f455302b35eaeabc616ab0d2"
expected_plan_two_hash="67c478a27498eb91a1aa22c87da077de33b991e0b1144dfb6c72fe8af550a658"

check_hash() {
    local path="$1"
    local expected="$2"
    local actual
    actual="$(sha256sum "$path" | cut -d' ' -f1)"
    if [[ "$actual" != "$expected" ]]; then
        echo "plan lock mismatch: $path" >&2
        echo "expected $expected" >&2
        echo "actual   $actual" >&2
        return 1
    fi
}

check_sequence() {
    local path="$1"
    local prefix="$2"
    local first="$3"
    local last="$4"
    local width="$5"
    local found
    local expected
    local missing

    found="$(
        rg -o "\\b${prefix}[0-9]{${width}}\\b" "$path" \
            | sort -u
    )"
    expected="$(
        for ((index = first; index <= last; index += 1)); do
            printf "%s%0*d\n" "$prefix" "$width" "$index"
        done
    )"
    if [[ "$found" != "$expected" ]]; then
        missing="$(comm -23 <(printf '%s\n' "$expected") <(printf '%s\n' "$found"))"
        echo "board lock mismatch: $path does not contain the exact $prefix sequence" >&2
        if [[ -n "$missing" ]]; then
            echo "missing:" >&2
            printf '%s\n' "$missing" >&2
        fi
        return 1
    fi
}

check_embedded_copy() {
    local board="$1"
    local source="$2"
    local begin_marker="$3"
    local end_marker="$4"
    local begin_count
    local end_count

    begin_count="$(rg -F -x -c "$begin_marker" "$board" || true)"
    end_count="$(rg -F -x -c "$end_marker" "$board" || true)"
    if [[ "$begin_count" != "1" || "$end_count" != "1" ]]; then
        echo "embedded board lock mismatch: expected one marker pair for $source" >&2
        return 1
    fi
    if ! cmp -s \
        <(
            awk -v begin="$begin_marker" -v end="$end_marker" '
                $0 == begin { capture = 1; next }
                $0 == end { exit }
                capture { print }
            ' "$board"
        ) \
        "$source"
    then
        echo "embedded board lock mismatch: $board does not contain an exact copy of $source" >&2
        return 1
    fi
}

check_hash "$plan_one" "$expected_plan_one_hash"
check_hash "$plan_two" "$expected_plan_two_hash"

check_embedded_copy \
    "$board_one" \
    "$plan_one" \
    "<!-- BOARD_EMBED_PLAN_ONE_BEGIN -->" \
    "<!-- BOARD_EMBED_PLAN_ONE_END -->"
check_embedded_copy \
    "$board_one" \
    "$plan_two" \
    "<!-- BOARD_EMBED_PLAN_TWO_BEGIN -->" \
    "<!-- BOARD_EMBED_PLAN_TWO_END -->"
check_embedded_copy \
    "$board_one" \
    "$board_two" \
    "<!-- BOARD_EMBED_PLAN_TWO_BOARD_BEGIN -->" \
    "<!-- BOARD_EMBED_PLAN_TWO_BOARD_END -->"
check_embedded_copy \
    "$board_one" \
    "$merge_board" \
    "<!-- BOARD_EMBED_BRANCH_MERGE_BOARD_BEGIN -->" \
    "<!-- BOARD_EMBED_BRANCH_MERGE_BOARD_END -->"

check_sequence "$board_one" "P1." 1 45 2
check_sequence "$board_one" "P1-C" 1 4 2
check_sequence "$board_two" "P2." 1 36 2
check_sequence "$board_two" "GUI-R" 1 26 2
check_sequence "$board_two" "GUI-C" 1 12 2
check_sequence "$board_two" "P2-G" 1 9 2
check_sequence "$board_one" "P2." 1 36 2
check_sequence "$board_one" "GUI-R" 1 26 2
check_sequence "$board_one" "GUI-C" 1 12 2
check_sequence "$board_one" "P2-G" 1 9 2

echo "Leader-AI plan locks are complete:"
echo "  Main board contains exact embedded copies of both plans and both merge boards"
echo "  Plan 1 hash and main-board P1.01-P1.45/P1-C01-P1-C04"
echo "  Plan 2 hash and main-board P2.01-P2.36/GUI-R01-GUI-R26/GUI-C01-GUI-C12/P2-G01-P2-G09"
