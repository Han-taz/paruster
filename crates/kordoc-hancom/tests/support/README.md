# HWPX synthetic fixture provenance (H0)

These archives are generated from [`hwpx_fixture.rs`](hwpx_fixture.rs), licensed
CC0-1.0. The generator source SHA-256 is
`9d8454364c1e432141133ee6b148b13c29d0546ace776b9583bcf5de77e6c9a1`.
It creates fixed-order ZIP records with the 1980-01-01 00:00:00 timestamp,
stored payloads, and explicit UTF-8 XML. No bytes from the ignored migration
oracle or its fixtures are copied into an archive. Generated ZIPs are not
committed; Rust tests regenerate and hash them. The fixed encryption password
is `fixture-password`, with salt `11` repeated 16 times, IV `22` repeated 16
times, and 1,024 PBKDF2 iterations. The `encrypted_sha1` and
`encrypted_sha256` names identify the HMAC PRF used to derive the ciphertext.
ODF 1.2 does not declare that PRF in the manifest; the oracle tries SHA-1 then
SHA-256. The test independently decrypts each with its named PRF, rejects the
other PRF, and checks the recovered XML. The manifest declares AES-256-CBC,
SHA-256 start key, and `sha256-1k` checksum.

| Recipe | Input SHA-256 | Evidence use |
| --- | --- | --- |
| `minimal` | `edf2d5dea68e88ff2c101df035bc6b8487a71100c8709b85276ba89054c3ae98` | Full result candidate |
| `two_section_spine_reversed` | `4f3a9fcbaad1156c0b8f53622369bbef19a924bcb6e384fa730efa49e38d9b9d` | Full result candidate; spine order |
| `nested_table` | `1ad3b674850b0b87d742bce66775d1e5637d9cb14bc08677a8d1f95fbd470243` | Full result candidate; recursive table |
| `page_cache` | `62c89c7f0dac1c00fd15199692825bbaaa8b17a75de41e4224831c94c8b7b58a` | Full result candidate; layout pages |
| `missing_page_cache` | `5e6d6dd4ef537d993a5d7f0296434a4d0939806dbbbf87d9bd3c7262c4efd4e6` | Full result candidate; section pages |
| `encrypted_sha1` | `4cfd465818a4ac733d6333543d0930f912287642a77a006aecbc95df5e1d306f` | Full result candidate; PBKDF2 HMAC-SHA1 |
| `encrypted_sha256` | `a579b3fa7c0d1b4fea4eedf327ad4e2a1bd21b0755c97ae03b9403d5663a306f` | Full result candidate; PBKDF2 HMAC-SHA256 fallback |
| `malformed_section` | `2dac442e805b8cfc1baf523899dae2906fc274be05c662f7131e58436f19ab00` | Full result candidate; section warning |
| `over_depth_section` | `7bb57a6512ea2170cd16ce4f0ecd4299344e4254072476f28e9d5f47aaf4a2d9` | Security divergence, excluded from parity numerator |
| `over_depth_manifest` | `1e1944755e0cd9c255c76e7e620665de71e57df3f2dba115a459a9f683f33c7a` | Security divergence, excluded from parity numerator |
| `directory_entry_500` | `cbdb78eb1eebf5f402495ec23639687446e414b9f90e85dbd2ba6b360c631277` | Safety boundary, excluded from parity numerator |
| `directory_entry_501` | `2813d4bad568a2147394ef70a939d85e46428686597d8ffe914891f951c629ba` | Safety boundary, excluded from parity numerator |

The 500/501 archives contain exactly one file (`mimetype`) and respectively
499/500 explicit directory records. They intentionally have no sections so
record counting is isolated from successful parsing. The oracle currently
returns `NO_SECTIONS` at 500 and `ZIP_BOMB` at 501. For over-depth section XML
the oracle returns a success with empty blocks, while the approved Rust design
requires a `PARTIAL_PARSE` warning; for over-depth critical manifest XML the
oracle returns success, while the Rust design requires `CORRUPTED`. These are
explicit non-parity security cases. A corrupt central directory's strict
refusal is also an approved non-parity boundary; H1a will provide its own
adversarial bytes and regression test. None of these observations changes
golden answers, scoring, or the two permitted normalizations.

The read-only oracle source was commit
`bb71f7fb0bf51dd456d27505a8c04772df182144`. Source SHA-256:

| File | SHA-256 |
| --- | --- |
| `src/index.ts` | `85943942619d9c2b7ed0855bdb58eb0360fec028e84efdd0f719589ba6ce4e01` |
| `src/hwpx/parser.ts` | `798864fcb7cc929f2fa4481ebbcf9d3e208fc22a87a2d0803854ab6d8e1768fa` |
| `src/hwpx/crypto.ts` | `9bed93ff9a3076e1700246fc04fea4df4ec19607ee79d2659adb7ba0628ee016` |
| `src/hwpx/zip-sections.ts` | `e4176d440acfbfe9a37f92a21b703dba007bbb7663bae5eac621cabce11e4539` |

[`hwpx-oracle-results.jsonl`](hwpx-oracle-results.jsonl) contains the full
flat public `parse()` result for every recipe, without normalization. Its
SHA-256 is `edfdb1c0966d56579d84a4b38c6d09bf306bdbad8527952c937b8e99a5df13fc`.
Eight semantic results are candidate H4 goldens; H0 alone does not advance the
parser parity numerator or make a capability claim. The last four lines are
security observations only.

Local capture used these commands; the temporary copy allowed package imports
while the ignored oracle checkout stayed read only. The `npm install` line
supplied `tsx`'s platform binary after omitting optional dependencies:

```sh
PARUSTER_HWPX_FIXTURE_EXPORT=/Users/shkoh/Projects/paruster-worktrees/parse-hwpx/target/hwpx-fixtures cargo test -p kordoc-hancom --test hwpx_integration --locked export_generated_fixture_bytes_for_oracle_capture
mkdir -p /tmp/paruster-hwpx-oracle.ck4d9v
cp -R /Users/shkoh/Projects/paruster/kordoc/src /tmp/paruster-hwpx-oracle.ck4d9v/src
cp /Users/shkoh/Projects/paruster/kordoc/package.json /Users/shkoh/Projects/paruster/kordoc/package-lock.json /tmp/paruster-hwpx-oracle.ck4d9v/
cd /tmp/paruster-hwpx-oracle.ck4d9v
npm ci --omit=optional --ignore-scripts --no-audit --no-fund
npm install --no-save --ignore-scripts --no-audit --no-fund @esbuild/darwin-arm64@0.28.2
node --import tsx --input-type=module -e 'import fs from "node:fs/promises"; import { parse } from "./src/index.ts"; const ids = ["minimal", "two_section_spine_reversed", "nested_table", "page_cache", "missing_page_cache", "encrypted_sha1", "encrypted_sha256", "malformed_section", "over_depth_section", "over_depth_manifest", "directory_entry_500", "directory_entry_501"]; for (const id of ids) { const bytes = await fs.readFile("/Users/shkoh/Projects/paruster-worktrees/parse-hwpx/target/hwpx-fixtures/" + id + ".hwpx"); const options = id.startsWith("encrypted_") ? {password:"fixture-password"} : undefined; console.log(JSON.stringify({id, result:await parse(bytes, options)})); }'
```

The machine-readable capture has one object per line, each containing the
recipe `id` and the entire `result`. H4 may relocate approved goldens into
the coordinator-owned document manifest only after parser and installed-wheel
comparisons pass. H3 publishes the normative HWPX component page.
