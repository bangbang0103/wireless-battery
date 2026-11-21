## Summary

## Verification

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-features`
- [ ] `wireless-battery validate` and relevant replay fixtures pass
- [ ] Automatic checks and real-hardware checks are reported separately

## Device and safety checklist

- [ ] Discovery and static matching perform no device I/O
- [ ] Matching conflicts cannot select an arbitrary provider
- [ ] Normal, malformed, invalid-field, and relevant timeout/fallback fixtures are included
- [ ] Unknown values are not converted to zero, not-charging, or offline
- [ ] Synthetic evidence is marked and excluded from real support
- [ ] Provenance and licenses are recorded; `verified` has real-hardware evidence
- [ ] No serial numbers, raw paths, usernames, PCAP, keystrokes, or unrelated traffic
- [ ] Capability matrix states only what evidence supports
- [ ] A simple YAML device required no core Rust changes, or the abstraction issue is explained
