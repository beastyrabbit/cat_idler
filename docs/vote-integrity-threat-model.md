# Vote Integrity Threat Model

Refs #9.

## Path

Vote actions enter through `app/api/game/actions/route.ts`. The route verifies the HMAC-signed session for `castVote` and `requestVoteKick`, derives a salted subscriber hash from trusted platform IP headers, then calls `server/elections.ts`.

By default the subscriber hash ignores client-controlled `x-forwarded-for`; deployments that sanitize and trust a specific header can opt in with `TRUSTED_SUBSCRIBER_IP_HEADER`.

`server/elections.ts` upserts the player row by signed session, requires a minimum voting history, and stores one row in `votes` for either matching identity axis:

- `votes_by_election_player`: one ballot per verified session/player row.
- `votes_by_election_subscriber`: one ballot per salted subscriber/network hash.

`runElectionLifecycle` reads `votes`; `tallyVotes` and `shouldTriggerKick` count effective identities, preferring subscriber hash when present and falling back to player id for legacy/null-hash rows. Vote-kick still requires 5 effective identities.

## Before

The HMAC session work prevented impersonating an existing voter, but vote rows deduped only by `electionId + playerId`, and `players` deduped only by session id. The IP-based subscriber hash in `app/api/subscriber-hash/route.ts` was display-only.

Attack cost was low:

- One IP could mint fresh sessions and cast fresh player votes. The route limiter allowed 30 unproven presence attempts per 10 seconds per IP, then 30 actions per 10 seconds per verified session. A 5-signature vote-kick could be flipped with a small loop inside the 10-minute kick window.
- A modest IP pool made the same attack easier to parallelize. The old controls were spam brakes, not vote-integrity boundaries.

## After

Session rotation from one network identity collapses to one ballot per poll. A new voting session must also have at least two recorded presence events and be at least 2 minutes old.

Attack cost now:

- One IP / one subscriber hash: at most 1 effective vote in an election or vote-kick, regardless of how many signed sessions are minted. It cannot flip a 5-vote kick by itself.
- Modest IP pool: needs at least 5 distinct subscriber hashes, each with a signed session warmed for at least 2 minutes and at least two presence records, before the 10-minute kick window closes.

Accepted tradeoff: NAT-shared households collapse to one effective ballot; the latest vote from that household identity wins. If a deployment loses trusted real-client IP headers, all unknown clients share the hash for `unknown` until proxy forwarding is fixed. Configure `SUBSCRIBER_HASH_SALT` in production so stored subscriber hashes are deployment-specific, and only set `TRUSTED_SUBSCRIBER_IP_HEADER` to a header your edge/proxy strips and rewrites.

Known gap: this is not account-grade Sybil resistance. A prepared attacker with enough distinct IPs/proxies can still assemble 5 warmed identities. Raising the kick threshold or adding account/attestation signals would be the next step if vote-kick becomes valuable enough to defend against proxy-pool attackers.
