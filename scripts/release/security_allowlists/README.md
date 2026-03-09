# Supply Chain Allowlists

These CSV files are consumed by `scripts/release/supply_chain_security_gate.sh`.

Rules:
- Every row must include `expires_on` in `YYYY-MM-DD` format.
- Expired rows make the gate fail.
- `owner` and `reason` are mandatory.
- Use the shortest expiry window possible and rotate/remove rows after remediation.

## cargo_deny_advisories_allowlist.csv
Schema:
`advisory_id,target,expires_on,owner,reason`

- `advisory_id`: RustSec advisory ID, e.g. `RUSTSEC-2023-0071`
- `target`: Rust workspace dir (e.g. `chat`) or `*`

## pip_audit_vulns_allowlist.csv
Schema:
`vuln_id,package,expires_on,owner,reason`

- `vuln_id`: `GHSA-*`, `PYSEC-*`, or `CVE-*`
- `package`: affected package name in `requirements.txt`
