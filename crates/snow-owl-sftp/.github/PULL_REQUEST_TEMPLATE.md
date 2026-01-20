# Pull Request

## Description

<!-- Provide a brief description of the changes -->

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Security fix

## Compliance Checklist

### Required (MUST be completed)

- [ ] Code formatted with `cargo fmt --package snow-owl-sftp`
- [ ] All clippy warnings resolved (`cargo clippy --package snow-owl-sftp -- -D warnings`)
- [ ] All tests pass (`cargo test --package snow-owl-sftp`)
- [ ] Documentation tests pass (`cargo test --doc --package snow-owl-sftp`)
- [ ] `./verify.sh` passes without errors
- [ ] NIST 800-53 controls documented for security-relevant code
- [ ] Application Security STIG findings addressed
- [ ] docs/CHANGELOG.md updated with changes
- [ ] Relevant documentation updated (README, docs/RFC_COMPLIANCE.md, etc.)

### NIST 800-53 Controls

**If this PR involves security-relevant code, list applicable controls:**

- [ ] AC-2: Account Management
- [ ] AC-3: Access Enforcement
- [ ] AC-7: Unsuccessful Logon Attempts
- [ ] AC-11: Session Lock
- [ ] AC-12: Session Termination
- [ ] AU-2: Audit Events
- [ ] AU-3: Content of Audit Records
- [ ] IA-2: Identification and Authentication
- [ ] SC-8: Transmission Confidentiality
- [ ] SC-13: Cryptographic Protection
- [ ] SI-10: Information Input Validation
- [ ] SI-11: Error Handling
- [ ] Other: _____________

### STIG Compliance

**If this PR addresses STIG findings, list them:**

- [ ] V-222396: Input validation
- [ ] V-222566: Secure error messages
- [ ] V-222577: Cryptographic mechanisms
- [ ] V-222596: Authorization enforcement
- [ ] V-222601: Session termination
- [ ] V-222602: Session ID protection
- [ ] V-222611: Certificate validation
- [ ] Other: _____________

## Documentation Updates

**Check all that apply:**

- [ ] README.md
- [ ] docs/RFC_COMPLIANCE.md
- [ ] docs/ROADMAP.md
- [ ] docs/QUICKSTART.md
- [ ] docs/SECURITY.md
- [ ] docs/CHANGELOG.md
- [ ] Code documentation (rustdoc)
- [ ] config.example.toml
- [ ] No documentation updates needed

## Testing

**Describe the testing you've done:**

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Tested with standard SFTP clients (openssh, FileZilla, etc.)
- [ ] Performance testing (if applicable)
- [ ] Security testing (if applicable)

**Test scenarios covered:**

1.
2.
3.

## Security Impact

**Does this PR have security implications?**

- [ ] Yes (describe below)
- [ ] No

**If yes, describe:**

- Security controls implemented:
- Threat model considerations:
- Attack vectors mitigated:

## Breaking Changes

**Does this PR introduce breaking changes?**

- [ ] Yes (describe below and update docs/CHANGELOG.md)
- [ ] No

**If yes, describe:**

- What breaks:
- Migration path:
- Deprecation timeline:

## Verification

**Paste the output of `./verify.sh`:**

```
[Paste output here]
```

## Roadmap Impact

**Does this PR complete any roadmap items?**

- [ ] Yes (specify which phase/task)
- [ ] No

**Roadmap reference:** Phase X.Y - Task Name

## Additional Notes

<!-- Any additional information, context, or screenshots -->

## Checklist for Reviewers

**Reviewers should verify:**

- [ ] Code follows docs/DEVELOPMENT_RULES.md
- [ ] NIST/STIG comments present on security code
- [ ] All documentation is updated and accurate
- [ ] No clippy warnings
- [ ] No formatting issues
- [ ] Tests are comprehensive
- [ ] Security implications reviewed
- [ ] Breaking changes documented

---

**By submitting this PR, I confirm that:**

- I have read and followed docs/DEVELOPMENT_RULES.md
- All code includes required NIST 800-53 and STIG compliance comments
- All code passes `cargo fmt` and `cargo clippy` with zero warnings
- All relevant documentation has been updated
- This PR is ready for review

---

**Closes:** #(issue number)
**Related:** #(related issues/PRs)
