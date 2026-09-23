# lxp

A Rust command-line client for [LetterXpress](https://www.letterxpress.de/),
using its [v3 API](https://www.letterxpress.de/versandwege/api).

## Development

The Nix environment is based on `github:mbr/flakes#rust`. Run `direnv allow`,
then `./check.sh` for checks and tests and `./format.sh` for formatting.
`nix build` produces `result/bin/lxp`. CI builds and tests through Nix.

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
retrying. `--notice` is a correlation reference, not deduplication.

A directory passed to `send` submits its PDFs sequentially and stops on failure.
`watch-dir DIRECTORY` watches completed writes/renames, archives confirmed files
under `sent`, and stops on errors or repeated paths. Event support varies by OS;
manual `send` is preferable. Neither command recursively scans directories.

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
