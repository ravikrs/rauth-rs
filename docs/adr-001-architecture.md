# ADR-001 — Architecture

**Status:** Accepted
**Date:** 2026-11-27

## Context

`rauth-rs` is an OAuth 2.0 / OpenID Connect provider in Rust. The target is a multi-tenant provider that serves millions of users, passes the OpenID conformance
suite, and is operated through an admin API and a console.

Inputs: the RFCs, and the references in `reference/README.md`: Rust providers and libraries, and
the Go projects Hydra, Kratos, Dex and Zitadel for protocol and domain design. The service shape
is modelled on Ory Hydra and Kratos. Paths below are relative to `reference/src/`.

## Decisions

### D1 — Four bounded contexts, deployed by load profile

| Context | Owns |
|---|---|
| `clients` | Client registration: identifiers, redirect URIs, grant types, secrets, keys |
| `tokens` | The protocol: authorize and token endpoints, grants, issuance, introspection, revocation, signing keys. **No user record, no credential** |
| `users` | Users, credentials, verification and recovery codes (each its own aggregate) |
| `courier` | Delivery of email and SMS queued by other contexts |

Each context is a set of crates (D4). Deployables are a separate choice from contexts:

| Deployable | Contains | Why it stands alone |
|---|---|---|
| `rauth-tokens` | `tokens` + `clients` + key management | High volume, cheap per request, latency-sensitive |
| `rauth-users` | `users` | Bursty and expensive: password hashing is deliberately slow and memory-heavy |
| `rauth-courier` | `courier` | A worker with no inbound API; slow third-party calls must not hold requests |
| `rauth-console` | The console (D9) | A client of the admin API; different release cadence and audience |
| `rauth-allinone` | Everything | Development, the conformance suite, small installs. Not a role flag: a separate binary crate |

**Clients live with tokens.** Hydra keeps `client/`, `oauth2/` and `jwk/` in one deployable
(`hydra/client/`, `hydra/oauth2/`, `hydra/jwk/`, verified). Every token request reads the client;
a network hop on that path buys nothing.

**Users are separate from tokens.** Hydra has no user, identity or credential package; Kratos owns
identity and its self-service flows, recovery and verification included (top-level packages of
`hydra/`; `kratos/selfservice/flow/recovery/`, `verification/`; both verified). The stronger reason here is load: a credential-stuffing
attack on `users` must not starve token refreshes for users already signed in.

**Courier is a worker, not a service.** Kratos ships its courier in the same codebase and runs it
as a separate process with `kratos courier watch` (`kratos/cmd/courier/watch.go:23`, verified).
Contexts queue messages through their outbox (D7); nothing calls the courier.

**No events service.** No surveyed project runs one. Events are an outbox per service (D7).

**Rejected: a deployable per context.** It turns the client lookup on every token request into a
network call and gives three services one database, which moves the bottleneck without adding
capacity (D2).

### D2 — Scaling model

**Load estimate** (assumptions, to be replaced by measurement): resource servers verify JWT access
tokens locally against a cached JWKS, so API traffic does not reach the provider. What does:

| Operation | Frequency per user | Cost |
|---|---|---|
| Login | Once per session (hours to days) | Argon2id: tens of ms and ~19 MiB at OWASP minimum parameters (*recalled*) |
| Token refresh | Once per access-token lifetime | Cheap CPU; **one write** for rotation and reuse detection |
| JWKS, discovery | Cached by clients | Static; CDN-cacheable |
| Introspection, userinfo | Per resource-server policy | Reads; cacheable |

One million concurrently active users at a ten-minute access-token lifetime is ~1,700 refreshes
per second on average; plan for 5–10x at peak.

**Rules that follow:**

- **Every deployable is stateless**; capacity comes from replicas behind a load balancer.
- **Shared state is the bottleneck** — refresh tokens, authorization codes, sessions. Design those
  tables for write volume and expiry from the start; expired rows are purged by a job (Hydra ships
  one: `hydra/cmd/cmd_delete_tokens.go`, verified to exist).
- **Hot paths split by routing, not by code.** The ingress sends the token endpoint, introspection
  and JWKS to separate replica pools of the same binary.
- **CPU-bound work never runs on the async runtime.** Password hashing and RSA signing go to a
  bounded blocking pool (`spawn_blocking` behind a semaphore), so a login storm saturates the pool,
  not the reactor. Load beyond the pool is shed with `503` and `Retry-After`.
- **Prefer EdDSA/ES256 signing** over RSA for issued tokens; RSA signing is an order of magnitude
  costlier (*recalled*). RS256 stays available because OIDC requires it.

**Signals to split further**, and what to do:

- Login and consent need a different release or scaling cadence from the token endpoint → split the
  browser-facing login UI out, as Hydra delegates login and consent (`hydra/consent/strategy.go`)
  and Zitadel ships its login as a separate app (`zitadel/apps/login`, verified).
- Refresh-token writes saturate the database → partition by tenant (D3 makes it possible) or move
  token state to a dedicated store. Not more services.

### D3 — Multi-tenancy from the first commit, by construction

Every aggregate carries `TenantId` (the isolated boundary) and `OwnerId` (the organisation within
it). A repository can only be constructed from a `Scope { tenant, owner }` value with private fields,
so a handler cannot express a cross-tenant query (IronAuth `crates/ironauth-store/src/scope.rs:21`).
Postgres row-level security is the second layer, set per transaction
(`crates/ironauth-store/src/replication.rs:302`).

A cross-tenant test family runs against every endpoint, plus a ratchet test that freezes the set of
files allowed to resolve a tenant by hand. FerrisKey added both after a critical cross-realm advisory
(`api/tests/realm_scope_ratchet_test.rs`; advisory *claimed* in its docs).

### D4 — Crates enforce dependency direction

```
crates/
  rauth-<ctx>-domain    value objects, aggregates, port traits. No I/O, serde, web or SQL crates
  rauth-<ctx>-postgres  driven adapter
  rauth-<ctx>-memory    driven adapter, also the test double
  rauth-<ctx>-http      driving adapter: handlers and wire types
  rauth-<ctx>-api       generated client for consumers (D6)
bins/
  rauth-tokens  rauth-users  rauth-courier  rauth-console  rauth-allinone
```

A driving adapter depends on the port, never on another adapter; only `bins/` name concrete
adapters. In Rust a crate boundary is compiler-enforced, so no linter is needed to hold it. FerrisKey
keeps its domain crate free of ORM and driver dependencies; oauth-as keeps Postgres in its own crate
to prove the seam usable from outside (`reference/README.md`).

**A CI check fails if a domain crate's dependency tree contains an I/O, web, SQL or serde crate.**
It is kept only after it has been seen rejecting a deliberately bad dependency.

Each binary links only the crates it uses, so an SMTP CVE does not ship in `rauth-tokens`.

### D5 — Domain types: private fields, smart constructors, no serialisation

Fields are private; values are built through constructors that validate, so an invalid value cannot
be constructed. Every protocol string is a newtype; secret types redact `Debug` (openidconnect-rs
`src/macros.rs`). Domain crates derive no `serde`: wire shapes live in `-http` and `-api` crates and
are mapped by hand.

Rust's encoders do not need public fields, so nothing forces the domain to expose them. FerrisKey derives `serde` and `utoipa` on its domain types; that couples the domain to the wire
format, which D6 forbids.

### D6 — Wire contracts are spec-first; consumers define narrow traits

Protocol endpoints are defined by the RFCs. The admin API is an OpenAPI document checked into the
repository and is the source of truth; server stubs and the `-api` client crates are generated from
it and never hand-edited. A consumer defines the narrow trait it needs, a thin wrapper over the
generated client, and a fake, and maps the generated DTO into its own types — so a provider adding a
field changes no consumer (modelled on `kratos/hydra/`, verified).

### D7 — State-based persistence with a transactional outbox

One storage port per context; a `-postgres` and a `-memory` implementation; one conformance kit both
must pass, shipped behind a `test-util` feature and including race checks (oauth-as
`crates/oauth-as/src/storage_conformance.rs`, the Rust form of Dex's `storage/conformance/`). Port
contracts follow oauth-as `crates/oauth-as/src/store.rs:476`: single-use artefacts are `take_*`
(atomic remove-and-return), read-modify-write is `compare_and_swap_*`, and adapters retry transient
contention internally.

Ports are traits whose methods return `impl Future<Output = …> + Send`, not `async fn` (which cannot
promise `Send` to callers) and not `async_trait`. FerrisKey and oauth-as do this; oxide-auth chose
sync ports and had to re-declare them in `oxide-auth-async`.

**Every state change writes an outbox row and an append-only audit row in the same transaction**
(IronAuth `docs/adr/0002-relational-primary-store.md`). The outbox feeds the courier and any future
consumer; failed credential checks are audit rows from the first commit, so brute-force detection is
a query, not a counter.

Event sourcing one aggregate remains a later exercise. IronAuth argues against it
citing a Zitadel post-mortem (zitadel#9599, *claimed*, not read); read it before starting.

### D8 — Keys are ours; JOSE is a narrow crate on vetted primitives

A `rauth-jwk` crate owns key generation, storage, rotation, active-key selection and the JWKS
endpoint, following Hydra's split (`hydra/jwk/`). KMS and HSM backends sit behind a Cargo feature,
so the default build carries no PKCS#11 dependency.

A `rauth-jose` crate implements JWS itself, on `aws-lc-rs`, with no high-level JOSE crate:

- **JWS compact serialization only.** Sign with EdDSA (default) and ES256; RSA (RS256/PS256) is
  verify-only, for clients that use RSA keys. JWE is out until a profile requires it.
- **The algorithm comes from the key, never from the token header.** Verification takes a
  per-client allowlist; `alg: none`, `jwk`, `jku` and `x5u` headers are rejected. This closes the
  algorithm-confusion and header-supplied-key class of bugs by construction.
- **Signing is a trait**, so a KMS or HSM backend signs without the private key entering the
  process.
- **Tested against RFC 7515, 7518 and 8037 vectors**, and differentially against `jsonwebtoken`,
  used only as a test oracle in dev-dependencies.

The references that take correctness most seriously do the same. IronAuth hand-rolls on `ring`
because high-level JOSE crates "carry the very CVE classes this core closes"
(`ironauth/crates/ironauth-jose/Cargo.toml:18`). oauth-as builds on `aws-lc-rs`, `p256`, `rsa` and
`ed25519-dalek`, and uses `jsonwebtoken` only in its conformance tooling, "never a dependency of the
published AS" (`oauth-as/crates/oauth-as-conformance/Cargo.toml:29-33`). Rauthy hand-rolls on
`ring`, `rsa` and `ed25519-compact`. FerrisKey, gtid and flipid use `jsonwebtoken`. All verified
from their `Cargo.toml`s.

**Rejected: `jsonwebtoken`.** It is general, which is where JOSE bugs live, and the hand-rolled
surface is small: base64url, compact serialization and a few algorithms over audited primitives.

### D9 — Administration: an admin API first, the console as its client

**Two listeners per deployable**, public and admin, on separate ports: Hydra's `hydra serve
public|admin` (`hydra/cmd/server/handler.go:51`, `:71`, verified). The admin listener is not exposed to
the internet by default.

**The admin API is protected by tokens rauth itself issues**: client credentials for automation,
authorization code + PKCE for people, authorised by scope and tenant. No static admin keys. The
first tenant and administrator are created by a `bootstrap` CLI subcommand that refuses to run twice.

**The console is a separate deployable that only calls the admin API.** It holds no database
credentials, and it is an ordinary OAuth client of rauth — so it exercises the provider's own login
flow on every use. Every console action is therefore an admin API call, authorised and audited like
any other; the console cannot do what the API cannot.

Precedents, verified as directories: Zitadel `console/` (Angular), Rauthy `frontend/` (SvelteKit),
Casdoor `web/` (React), Authelia `web/`. Open-source Hydra and Kratos ship no console; Ory's console
is part of its hosted product (*recalled*). Which stack the console uses is open (backlog).

### D10 — Naming

Repo `rauth-rs`; crates `rauth-*`; Rust API conventions (`snake_case` modules, no type names that
repeat the module: `client::Client`, `client::Id`).

## Consequences

- Seven crates per context is overhead before there is code to justify it; it is the price of
  compiler-enforced boundaries and per-binary dependency sets.
- Tenancy is intrusive from the first commit, before a second tenant exists.
- Every domain type needs a hand-written mapper to its wire shape.
- The outbox adds a write to every state change and a worker to operate.
- The admin API depends on the token service being up; the `bootstrap` CLI is the only path that
  does not.
- Owning the flows means owning their correctness. Mitigated by the OpenID conformance suite, the
  RFCs' own examples, and reading Fosite and Hydra as references.

## Open

- **Grant dispatch** — self-declaring handlers or an exhaustive `match`.
- **Application layer** — behaviour on the type, or FerrisKey's `ApplicationService`.
- **SSO session and login/consent ownership** — which context, and whether the login UI is its own
  deployable from the start.
- **RFC 8693 token exchange** — what to take from Dex, Ory, Zitadel and Rauthy.
- **Postgres or MySQL.** Every surveyed Rust provider with a relational store uses Postgres.
- **Console stack** — server-rendered Rust or a SPA.
- **OpenAPI generator** for Rust.

## Implementation phases

1. **`clients` domain.** Newtypes and the `Client` aggregate, memory adapter, conformance kit.
2. **`clients` adapters.** Postgres with RLS, admin API on the admin listener, `bootstrap` CLI.
3. **`tokens` core.** `client_credentials` and `authorization_code` + PKCE; `rauth-jose` and `rauth-jwk` with JWKS
   and rotation; `rauth-allinone`; the OpenID conformance suite running in CI.
4. **`users` and `courier`.** Credentials on the blocking pool, verification and recovery codes,
   outbox, audit rows, the courier worker.
5. **Console.** Client management first, signing in through rauth.
6. **Refresh rotation and token exchange.**
7. **Production.** KMS/HSM feature, observability, load test against D2's estimate, the split
   deployables and their routing.
