## Summary

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked --all-features`
- [ ] Automatic checks and real-hardware checks are reported separately

## Device and safety checklist

- [ ] Discovery and static matching perform no device I/O
- [ ] Matching conflicts cannot select an arbitrary provider
- [ ] Normal, malformed, invalid-field, and relevant timeout/fallback fixtures are included under `tests/fixtures/`
- [ ] Unknown values are not converted to zero, not-charging, or offline
- [ ] Synthetic evidence is marked and excluded from real support
- [ ] Licenses and attribution are preserved where applicable
- [ ] No serial numbers, raw paths, usernames, PCAP, keystrokes, or unrelated traffic
- [ ] `docs/support-matrix.md` and `docs/support-matrix.zh-CN.md` are updated
