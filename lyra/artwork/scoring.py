"""Similarity scoring for cover-art candidates (stdlib only)."""

import difflib
from .textutils import norm_key

#: Minimum total score (0-100) for a candidate to be accepted.
ACCEPT_THRESHOLD = 60.0
#: Above this the pipeline auto-picks without asking the user.
AUTO_PICK_THRESHOLD = 78.0

_VERSION_TOKENS = frozenset({
    "remix", "remaster", "remastered", "live", "acoustic", "instrumental",
    "karaoke", "cover", "version", "edit", "extended", "slowed", "sped",
    "reverb", "official", "audio", "video", "taylors",
})


def _tokens(key: str) -> set:
    return set(key.split()) if key else set()


def _token_set_ratio(a: str, b: str) -> float:
    """Rough port of token_set_ratio using difflib (0-100)."""
    if not a or not b:
        return 0.0
    if a == b:
        return 100.0
    ta, tb = _tokens(a), _tokens(b)
    if not ta or not tb:
        return 0.0
    inter = ta & tb
    if not inter:
        # Fall back to raw sequence similarity for transliterations.
        return difflib.SequenceMatcher(None, a, b).ratio() * 100.0
    # token_set: compare intersection against each side, take best.
    joint = " ".join(sorted(inter))
    only_a = " ".join(sorted(ta - tb)) or joint
    only_b = " ".join(sorted(tb - ta)) or joint
    s1 = difflib.SequenceMatcher(None, joint, a).ratio()
    s2 = difflib.SequenceMatcher(None, joint, b).ratio()
    s3 = difflib.SequenceMatcher(
        None, f"{joint} {only_a}".strip(), f"{joint} {only_b}".strip()
    ).ratio()
    return max(s1, s2, s3) * 100.0


def _version_mismatch_penalty(query_title: str, res_title: str) -> float:
    """Small penalty when one side is a remix/live/etc. and the other is not."""
    q = _tokens(norm_key(query_title)) & _VERSION_TOKENS
    r = _tokens(norm_key(res_title)) & _VERSION_TOKENS
    if bool(q) != bool(r):
        return 6.0
    if q != r and (q or r):
        return 3.0
    return 0.0


def score_candidate(
    query_artist: str,
    query_title: str,
    res_artist: str,
    res_title: str,
    provider_bonus: float = 0.0,
) -> float:
    """Total relevance score 0-100 (higher is better)."""
    q_t, r_t = norm_key(query_title), norm_key(res_title)
    if not q_t or not r_t:
        return 0.0
    title_sim = _token_set_ratio(q_t, r_t)
    # Hard gate: title must at least half-overlap, otherwise artist match
    # alone must not rescue a wrong song.
    q_tokens, r_tokens = _tokens(q_t), _tokens(r_t)
    overlap = len(q_tokens & r_tokens)
    title_ok = (
        (len(q_tokens) > 0 and overlap * 2 >= len(q_tokens))
        or q_t in r_t
        or r_t in q_t
        or title_sim >= 72.0
    )
    if not title_ok:
        return 0.0

    total = 0.62 * title_sim
    if norm_key(query_artist):
        a_sim = _token_set_ratio(norm_key(query_artist), norm_key(res_artist or ""))
        total += 0.33 * a_sim
    else:
        total += 0.33 * 50.0  # no artist info: neutral, title decides
    total -= _version_mismatch_penalty(query_title, res_title)
    total += provider_bonus
    return max(0.0, min(100.0, total))


def is_relevant(
    query_artist: str,
    query_title: str,
    res_artist: str,
    res_title: str,
    threshold: float = ACCEPT_THRESHOLD,
) -> bool:
    return score_candidate(query_artist, query_title, res_artist, res_title) >= threshold
