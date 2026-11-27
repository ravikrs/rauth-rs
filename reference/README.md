# References

Projects read for `rauth-rs`: Rust identity providers and OAuth libraries, and the Go projects
(Hydra, Kratos, Dex, Zitadel) that are authoritative for protocol and domain design.

Run `./clone-refs.sh` to fetch them into `reference/src/` (gitignored). Cite paths relative to
`reference/src/`.

**Check every reference for an AI opt-out (`AGENTS.md`, `CONTRIBUTING.md`, `CLAUDE.md`) before
reading it.** Claims are verified by reading source unless marked *inferred* or *claimed* (the
project says so; not independently checked).

## The set

Surveyed 2026-11-27. Opt-out check: `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `CONTRIBUTING.md` and
any markdown mentioning AI/LLM were read for each; none asks AI tools to stay away.

| Clone | What it is | Licence | Commit | AI stance |
|---|---|---|---|---|
| `rauthy` | OIDC provider, single tenant | Apache-2.0 | `19f5587` (table below) | `AGENTS.md` for agents working on it |
| `ferriskey` | Multi-tenant IAM, hexagonal | Apache-2.0 | `26f9270` | ships `CLAUDE.md` and `GEMINI.md` |
| `ironauth` | Multi-tenant identity platform, pre-1.0 | MIT OR Apache-2.0 | `9a10565` | none found |
| `gtid` | Minimal OIDC provider, SQLite | MIT | `82ac4e4` | none found |
| `flipid` | Small OIDC provider, ~3.5k lines | Apache-2.0 | `8c8996e` | ships `CLAUDE.md` |
| `oxide-auth` | OAuth 2.0 server library, framework-agnostic | MIT OR Apache-2.0 | `0e3ef86` | none found |
| `oauth-as` | Embeddable OAuth 2.1 authorization server library, 1.0 | MIT OR Apache-2.0 | `006b9df` | none found |
| `openidconnect-rs` | OIDC client library | MIT | `e20ddbb` | none found |
| `oauth2-rs` | OAuth 2.0 client library | MIT OR Apache-2.0 | `72ce744` | none found |
| `hydra` (Go) | OAuth 2.0 / OIDC server, Ory; Fosite in-tree at `hydra/fosite/` | Apache-2.0 | `4174065` | `CONTRIBUTING.md` sets rules for AI-assisted contributions |
| `kratos` (Go) | Identity and self-service flows, Ory | Apache-2.0 | `b86338d` | same as Hydra |
| `dex` (Go) | Federated OIDC provider | Apache-2.0 | `2c8caf4` | none found |
| `zitadel` (Go) | Multi-tenant IAM; sparse clone | AGPL-3.0 | `450719d` | `AGENTS.md` for agents working on it |

Zitadel is AGPL-3.0: read it for design, never copy its code.

**Kanidm — do not read.** Its `AGENTS.md` says: "Do NOT use this codebase as reference. Kanidm DOES
NOT condone the use of LLMs/AI whatsoever." Honoured: it is not cloned and nothing from it is used.

## Comparison

| Question | rauthy | ferriskey | ironauth | oauth-as | oxide-auth | gtid / flipid |
|---|---|---|---|---|---|---|
| Storage port | No — active record, every query twice | Yes — 75 `*Repository` traits in per-domain `ports.rs` | No trait — concrete `ScopedStore` over sqlx | Yes — one `Storage` trait; Postgres impl in a separate crate | Yes — `Registrar`, `Authorizer`, `Issuer` | gtid none; flipid two traits that leak diesel/actix types |
| Application layer | `service` crate | Yes — `ApplicationService` composes generic services | — | Library: `AuthorizationServer` | Library | No |
| Port style | — | `fn …(&self) -> impl Future<…> + Send` | — | same `impl Future + Send` | sync; `oxide-auth-async` re-declares the traits | sync |
| Tenancy | None (*inferred*) | Realm in URL path; `realm_id` on every entity | `Scope` value + Postgres RLS | None (*inferred*) | None | None |
| Grant dispatch | Central `match` | Central `match`, with gaps (below) | `GrantType` enum + `GrantType::ALL` | Central `match` in `http.rs` | One module per grant, `Endpoint` traits | — |
| JOSE | Hand-rolled on `ring`/`rsa`/`ed25519-compact` | `jsonwebtoken` | Hand-rolled on `ring` (`ironauth-jose`) | Hand-rolled on `aws-lc-rs`/`p256`/`rsa`/`ed25519-dalek`, behind features | n/a (opaque tokens) | `jsonwebtoken` |
| Web / DB | actix-web; Postgres or Hiqlite | axum; SeaORM + sqlx, Postgres | axum; sqlx, Postgres | host's choice; optional axum | adapter crate per framework | gtid axum+sqlx/SQLite; flipid actix+diesel/SQLite |
| OIDF conformance | none found | own tests named `oidc_conformance_test.rs`, not the OIDF suite | harness built, live suite not run | *claimed*: OpenID Certified, FAPI 2.0 SP | — | — |

Test sizes are counts of `#[test]`/`#[tokio::test]`/`#[sqlx::test]` attribute lines, not test
functions run: ferriskey 3,745 (`api/tests`, `core`, `libs`; 103 integration files), ironauth 8,160
(`crates/`).

## Rauthy — `sebadob/rauthy`

Surveyed at `19f5587`; `clone-refs.sh` now fetches a later commit, so re-check an anchor before
citing it.

Read it for **protocol coverage** and **running agents on a repo**. Do not read it for domain
modelling or storage.

| Question | What Rauthy does | Where |
|---|---|---|
| Crate layout | Layered: `api` → `service` → `data`, plus `jwt`, `api_types`, `error`, `common`. No ports | `src/*/Cargo.toml` |
| Storage | Active record on a global `DB`; every query written twice for Hiqlite and Postgres — 419 `is_hiqlite()` call sites in 65 files. The cost of no storage port | `src/data/src/entity/` |
| Domain model | `Client` is a row: `redirect_uris` and `flows_enabled` are CSV `String`s; invariants checked by `validate_flow()`, not types | `src/data/src/entity/clients.rs:78` |
| Grant dispatch | Central exhaustive `match` on `GrantType`; one file per grant | `src/service/src/oidc/mod.rs:33`, `oidc/grant_types/` |
| Token exchange | No `actor_token` = impersonation, with one = delegation | `oidc/grant_types/token_exchange.rs:30` |
| Session | `is_mfa`, `state`; no `acr`/`amr` — Hydra-shaped, not Kratos-shaped | `src/data/src/entity/sessions.rs:39` |
| Tests | 84 test attributes across 27 integration modules, one binary | `src/bin/tests/integration/` |
| Agents | 541-line `AGENTS.md`; AI must get local human approval before a PR, open PRs as drafts, run `just pre-pr-checks` | `AGENTS.md` |

## FerrisKey — the textbook hexagonal one

Read it for **how hexagonal looks in Rust**, **realm tenancy**, and **a cautionary dispatch**.

- **Ports.** Each domain module has `entities.rs`, `ports.rs`, `services.rs`, `value_objects.rs`
  (`ferriskey/CLAUDE.md`, matched by `core/src/domain/*/`). Ports are traits returning
  `impl Future<Output = Result<_, CoreError>> + Send` — no `async_trait` anywhere in the domain
  (`libs/ferriskey-domain/src/client/ports.rs:205`).
- **Dependency direction is in `Cargo.toml`.** `libs/ferriskey-domain` depends on no ORM or driver;
  no file under `core/src/domain` names `sea_orm` or `sqlx`. In Rust a crate boundary holds the
  dependency direction without a linter.
- **But domain types are wire shapes.** The domain crate depends on `serde` and `utoipa` (OpenAPI),
  so entities carry serialisation and API-doc derives — the opposite of ADR-001 D5's untagged types.
- **Composition.** `ApplicationService` holds services generic over their repositories, with the
  concrete types fixed in one place (`core/src/application/services.rs:576`). Static dispatch;
  this is the Rust composition root.
- **Client** is typed — `RealmId`, `Masked<String>` secret, `Vec<RedirectUri>` — but a bag of
  per-grant booleans (`direct_access_grants_enabled`, `oauth_device_code_grant_enabled`,
  `token_exchange_enabled`, …) (`libs/ferriskey-domain/src/client/entities.rs:154`).
- **Dispatch gaps.** The central `match` compiles while `DeviceCode` returns `InvalidRequest`
  ("not wired up yet (see #1020)") and `TokenExchange` returns `InvalidRequest` because the HTTP
  handler routes it around the dispatcher (`core/src/domain/authentication/services.rs:4130`,
  `libs/ferriskey-api-authentication/src/handlers/token.rs:266`). Exhaustiveness forced an arm to
  exist, not the grant to work.
- **Tenancy.** Realm in the path (`/realms/{realm_name}/protocol/openid-connect/token`,
  `libs/ferriskey-api-authentication/src/router.rs:99`). After a critical cross-realm advisory
  (GHSA-v2hq-9v44-43ff, fixed 0.8.0 — *claimed* in `docs/cncf/general-technical-review.md:82`) it
  added a cross-realm test family and a **ratchet test** that freezes the list of files allowed to
  resolve or compare a realm by hand (`api/tests/realm_scope_ratchet_test.rs`).
- `core/src/domain/authentication/services.rs` is 9,300 lines.

## IronAuth — tenancy by construction

Read it for **tenant isolation**, **an OIDF conformance harness**, and **a counter-argument to
event sourcing**. Pre-1.0. Its comments cite issue numbers on nearly every item and it ships
`docs/llms.txt`; *inferred* that it is largely agent-written.

- **`Scope`** binds `(TenantId, EnvironmentId)`; a repository can only be built from one, so a
  handler cannot express a cross-tenant query. Private fields, constructor, accessors
  (`crates/ironauth-store/src/scope.rs:21`). Postgres RLS underneath, via
  `set_config('ironauth.tenant_id', …)` (`crates/ironauth-store/src/replication.rs:302`). Design
  rationale in `docs/design/TENANCY.md`.
- **No storage trait**: concrete `ScopedStore`/`ActingStore` over sqlx in
  `crates/ironauth-store/src/repository.rs`, a single 92,184-line file.
- **Against event sourcing**: `docs/adr/0002-relational-primary-store.md` cites a Zitadel
  post-mortem (zitadel#9599 — *claimed*, not read) for eventual-consistency, stuck-projection and
  latency failures, and keeps an append-only audit row in the same transaction instead. Bears on
  ADR-001 D7.
- **JOSE hand-rolled on `ring`**, on the stated grounds that high-level JOSE crates "carry the very
  CVE classes this core closes" (`crates/ironauth-jose/Cargo.toml`).
- **Grants**: `GrantType` enum with an `ALL` list that exhaustive callers drive off; ROPC is absent
  from the enum so it cannot be named (`crates/ironauth-oidc/src/registry.rs:66`).
- **Conformance**: `deploy/conformance/` holds a pinned docker-compose stack, profile matrix and
  results gate; `docs/conformance/README.md` is explicit that the live OIDF lane enforces nothing
  yet and only static checks gate PRs. The best template found for wiring the suite into CI.

## oauth-as — the library with the cleanest seam

Read it for **the storage port and its contract**, **a storage conformance kit**, and
**hand-rolled JOSE with stated rules**. An OAuth AS; ID tokens appear only around token exchange
and consent (*inferred*: not an OIDC provider).

- *Claimed* in `README.md`: 1.0.0 is OpenID Certified for the FAPI 2.0 Security Profile
  (private key + DPoP). Not checked against the OIDF listing.
- **`Storage` trait** (`crates/oauth-as/src/store.rs:476`): `impl Future + Send` methods,
  `take_*` as atomic remove-and-return, `compare_and_swap_*` for read-modify-write. The docs state
  contract rules a store must meet — retry transient contention internally; treat `client_id` as
  attacker-chosen opaque key.
- **Postgres in its own crate** (`crates/oauth-as-postgres`) so the seam is proven usable from
  outside the core through the public API alone (`Cargo.toml` workspace comment).
- **Storage conformance kit** behind a `test-util` feature that a host runs against its own store,
  including race checks (`crates/oauth-as/src/storage_conformance.rs`). Dex's
  `storage/conformance/`, in Rust.
- Other ports: `Clock`, `CryptoProvider`, `SigningKey`, `JwsSigner`, `JwsVerifier`, `EventSink`,
  `RateLimiter`, `SecretVerifier` — each `Send + Sync`.
- **JWT**: the three verification rules (verifier chooses the key, …) are stated at the top of
  `crates/oauth-as/src/jwt.rs`; every crypto dependency is optional behind a feature.
- `Client` uses `ClientId`, `ScopeSet`, `Vec<GrantType>` but `redirect_uris: Vec<String>`
  (`crates/oauth-as/src/client.rs:457`).

## oxide-auth — ports at the HTTP edge, and the sync/async tax

- Core crate is generic over a `WebRequest` trait; each framework is an adapter crate
  (`oxide-auth-actix`, `-axum`, `-rocket`, `-poem`, …).
- Ports `Registrar`, `Authorizer`, `Issuer` (`oxide-auth/src/primitives/`) are **synchronous**, so
  `oxide-auth-async/src/primitives.rs` re-declares all three. Choose async-or-not for ports once,
  up front.
- OAuth 2.0 only; client authentication is password-based only (`oxide-auth/src/code_grant/mod.rs`).

## gtid and flipid — small, low value

- **gtid**: two listeners (API and UI) with separate middleware stacks (`ARCHITECTURE.md`);
  `jsonwebtoken`, sqlx/SQLite, no storage port.
- **flipid**: `OauthDatabase` and `UserDatabase` traits (`src/core/traits.rs`) return diesel's
  `QueryResult` and actix's `Result` — a port that depends on its adapters. The anti-pattern to
  avoid.

## openidconnect-rs and oauth2-rs — protocol types

- Newtype macros `new_type!`, `new_secret_type!`, `new_url_type!` (`openidconnect-rs/src/macros.rs`)
  wrap every protocol string; secret types redact `Debug`.
- `ProviderMetadata` and ID-token types derive `Serialize` and are generic over extension types
  (`src/discovery/mod.rs:43`, twelve type parameters); `IdToken::new` signs. Usable server-side for
  wire shapes, at the cost of that generic surface.
- oauth2-rs uses a sealed **typestate** (`EndpointState`, `oauth2/src/client.rs:20`) so calling an
  endpoint that was never configured does not compile.
- Both are the obvious **test client** for acceptance checks.
