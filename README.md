# lxp

A Rust command-line client for [LetterXpress](https://www.letterxpress.de/),
using its [v3 API](https://www.letterxpress.de/versandwege/api).

## Development

The Nix environment is based on `github:mbr/flakes#rust`. Run `direnv allow`,
then `./check.sh` for checks and tests and `./format.sh` for formatting.
`nix build` produces `result/bin/lxp`. CI builds, lints, checks documentation and
tests through Nix. Checks use synthetic credentials and loopback HTTP servers.
Diagnostics use `tracing` on stderr, defaulting to `info`; `RUST_LOG` or `-v`
controls verbosity. Credentials and document payloads are never logged.

Copy `.env.example` to `.env` and supply `LXP_USERNAME` and `LXP_API_KEY`.
Direnv loads `.env`; plain `nix develop` does not. Secrets and `.env` variants
are ignored by Git. Never commit them or put credentials in Nix expressions.
The CLI consumes environment variables, not `.env` files directly.

## Usage

```sh
lxp balance
lxp price --pages 1
lxp --mode test send letter.pdf --notice my-reference
lxp jobs --filter draft --page 1
lxp status 12345
lxp cancel 12345
lxp invoice list --page 1
lxp invoice get 12345 --output invoice.pdf
```

Output is JSON, except cancellation acknowledgments and local profile commands.
Job and invoice lists return one page with pagination metadata. `done` means
processing completed, not delivered. Registered-mail tracking is returned when
available. Invoice downloads refuse to overwrite files.

Uploads default to black-and-white, simplex, domestic delivery. Use
`--color color`, `--duplex` and `--shipping international` as appropriate.
The PDF must already contain its destination address in the envelope window.
Use the provider's [templates](https://www.letterxpress.de/downloads), A4 portrait,
embedded fonts, flattened forms and no password restrictions. Local validation
only checks size and the PDF header; the provider validates printability.

`--mode test` explicitly overrides `LXP_MODE`. Test mode uses the real account's
shopping cart: it does not print or incur postage. The website can inspect,
delete or manually release these jobs. Unreleased drafts expire after seven days.

Live mode immediately enters paid processing and requires a separate `--yes`:

```sh
lxp --mode live send letter.pdf --yes
```

This confirmation also applies when live mode comes from `LXP_MODE`. Never use
live mode just to test an integration. Cancellation is generally available for
15 minutes, sometimes less near production cutoff; it is not a safety net.

The service has no documented idempotency key. Uploads are not automatically
retried. If an upload is unconfirmed, inspect recent jobs or the website before
retrying. `--notice` is a correlation reference, not provider-side deduplication.

Before each upload the client creates a private, durable receipt under the local
data directory at `lxp/submissions`. Use `--state-dir` or `LXP_STATE_DIR` to choose
another directory. Receipts contain processing mode and acknowledgement state,
not credentials, addresses or PDF contents. A pending receipt remains after an
error or interruption; a successful upload records the job ID atomically.
The default provider notice contains the local receipt reference.

The same PDF and account cannot be submitted twice in the same mode without
`--allow-duplicate`. This also covers renamed files and changed print options.
Only use that flag after reconciling earlier attempts; it does not prevent
provider-side duplicates. Keep the receipt directory across invocations and do
not bypass it to retry a timeout. Different machines do not share local receipts.

A directory passed to `send` submits its PDFs sequentially and stops on failure.
`watch-dir DIRECTORY` watches completed writes/renames, claims inputs in private
`.lxp-pending-*` directories, and archives confirmed files under `sent`. Errors
stop the watcher and leave claimed inputs for manual reconciliation. Producers
must finish writing before handing files over; atomic rename into the watched
directory is preferable. Event support varies by OS; manual `send` is safer.
Neither command recursively scans directories.

## Manual test upload

`tests/fixtures/test-letter.typ` contains a synthetic one-page document addressed
to the provider's published business address. It uses embedded fonts, no forms
or encryption, and places the recipient safely inside the provider's window
area rather than on its upper boundary. Compile and submit it explicitly:

```sh
nix shell nixpkgs#typst -c typst compile tests/fixtures/test-letter.typ /tmp/lxp-test.pdf
lxp --mode test send /tmp/lxp-test.pdf
lxp --mode test status JOB_ID
lxp --mode test balance
```

Replace `JOB_ID` with the returned ID. Confirm `status` is `draft`, check the
complete extracted recipient address, and verify the balance is unchanged.
Do not manually release this synthetic document. `lxp --mode test cancel JOB_ID`
removes the test job. Compiling the fixture does not call the service; submitting
it does. Automated tests never upload this PDF.

## Local profiles and migration

Environment credentials are preferred. If both variables are absent, the client
uses the selected profile in the platform configuration directory under
`lxp/lxp.toml`. Partial environment credentials are rejected rather than mixed
with a profile. `profile save NAME`, `profile list`, `profile select NAME` and
`profile delete NAME` maintain stored profiles. Saving writes credentials to disk.
Legacy profile URLs are ignored: requests always use the official HTTPS v3 host.

The v3 CLI replaces the old flag-based job and invoice commands with explicit
subcommands. `set` remains an alias for `send`. Quotes and individual job status
queries are now available. Pagination replaces the old implicit seven-day list.
Bulk cancellation is intentionally not exposed; cancel explicit IDs instead.

## License

MIT. See `LICENSE` for the original copyright notice.
