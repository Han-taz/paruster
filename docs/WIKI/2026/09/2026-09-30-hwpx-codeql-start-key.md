---
id: 2026-09-30-hwpx-codeql-start-key
date: 2026-09-30
status: reviewed
component: hwpx-security
issue: null
pr: https://github.com/Han-taz/paruster/pull/21
commit: e2da270
related_ssot: [docs/SSOT/components/hwpx.md]
related_decisions: []
---

# ODF password start-key CodeQL triage

PR #21's hosted CI, security workflow, wheel and fuzz gates pass, while the
CodeQL result check reports alert 6, `rust/weak-sensitive-data-hashing`, for
SHA-256 applied to the password in `crypto.rs`. Independent SOL review confirms
that this is a scoped false positive: the digest is the format-mandated start
key, not a stored or terminal password hash.

[ODF 1.3 Part 2](https://docs.oasis-open.org/office/OpenDocument/v1.3/OpenDocument-v1.3-part2-packages.html)
sections 4.6 and 4.16.6 specify UTF-8 password start-key generation with SHA-256;
sections 4.7 and 4.16.9 specify PBKDF2 derivation from that key. The code accepts
the reviewed SHA-256/PBKDF2 manifest identifiers, checks salt and iteration
parameters, then feeds the digest into salted PBKDF2-HMAC-SHA1/SHA256 before
AES-CBC decryption. No start-key digest is stored, compared as a password
verifier, or returned. Substituting a password-storage hash at that line would
break the format rather than fix a vulnerability.

The alert is classified as a false positive with an audit comment scoped to
that occurrence. The CodeQL query, severity threshold and required result gate
remain enabled. Eight crypto unit tests and the encrypted integration case
pass, including both PRFs and wrong-password/iteration limits. This does not
resolve the separate documented compressed-versus-decompressed checksum
interoperability gap; no claim of full standards or parser parity is made.
