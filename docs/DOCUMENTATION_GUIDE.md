# SolUPG Documentation Guide

This guide explains how to maintain and update documentation for SolUPG. **Every code change must be accompanied by documentation updates.**

---

## Documentation Structure

```
docs/
├── architecture/           # System architecture docs
│   └── overview.md         # High-level architecture
├── development/            # Development status & progress
│   └── CURRENT_STATUS.md   # Live development status
├── phase-1-onchain-programs/
│   └── README.md           # Phase 1 specs & progress
├── phase-2-routing-engine/
│   └── README.md           # Phase 2 specs & progress
├── phase-3-api-gateway/
│   └── README.md           # Phase 3 specs & progress
├── phase-4-clearing-reconciliation/
│   └── README.md           # Phase 4 specs & progress
├── phase-5-compliance-monitoring/
│   └── README.md           # Phase 5 specs & progress
├── phase-6-testing-deployment/
│   └── README.md           # Phase 6 specs & progress
└── DOCUMENTATION_GUIDE.md  # This file
```

---

## Documentation Rules

### Rule 1: Every PR Must Update Docs

Before submitting a PR, ensure you have updated:

| Change Type | Required Doc Updates |
|-------------|---------------------|
| New feature | Phase README, CHANGELOG, CURRENT_STATUS |
| Bug fix | CHANGELOG, relevant phase README if behavior changed |
| Breaking change | README, CHANGELOG, phase docs, migration guide |
| New program/instruction | Phase 1 README, architecture/overview.md |
| API change | Phase 3 README, SDK docs |
| Config change | Relevant phase README |

### Rule 2: Update CHANGELOG.md

Every PR must add an entry to `CHANGELOG.md` under `[Unreleased]`:

```markdown
## [Unreleased]

### Added
- New feature description (#PR_NUMBER)

### Changed
- Changed behavior description (#PR_NUMBER)

### Fixed
- Bug fix description (#PR_NUMBER)

### Removed
- Removed feature description (#PR_NUMBER)
```

### Rule 3: Update CURRENT_STATUS.md

After completing any task, update `docs/development/CURRENT_STATUS.md`:

```markdown
## Current Sprint

| Task | Status | Owner | Updated |
|------|--------|-------|---------|
| Implement create_payment | ✅ Done | @username | 2026-03-29 |
| Write unit tests | 🔄 In Progress | @username | 2026-03-29 |
| Deploy to devnet | ⏳ Pending | - | - |
```

### Rule 4: Update Phase README Checklists

Each phase has a deliverables checklist. Mark items as complete:

```markdown
## Deliverables Checklist

- [x] `solupg-payment` program with full test coverage
- [x] `solupg-escrow` program with full test coverage
- [ ] `solupg-splitter` program with full test coverage  <!-- Still in progress -->
```

---

## Documentation Templates

### New Feature Documentation

When adding a new feature, document:

1. **What** — What does it do?
2. **Why** — Why was it added?
3. **How** — How to use it (code example)
4. **API** — Function signatures, parameters, return values
5. **Events** — On-chain events emitted (if applicable)

### Code Comments

For Rust programs:
```rust
/// Creates a new payment intent.
///
/// # Arguments
/// * `ctx` - The program context containing accounts
/// * `amount` - Payment amount in token base units
/// * `recipient` - Recipient's wallet address
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(PaymentError)` on failure
///
/// # Events
/// Emits `PaymentCreated` event on success
pub fn create_payment(ctx: Context<CreatePayment>, amount: u64, recipient: Pubkey) -> Result<()> {
    // ...
}
```

---

## Commit Message Convention

Include doc updates in commit messages:

```
feat(payment): add timeout handling for pending payments

- Added 24h timeout for unclaimed payments
- Auto-refund to payer after timeout
- Updated Phase 1 README with new instruction
- Added CHANGELOG entry

Docs: phase-1-onchain-programs/README.md, CHANGELOG.md
```

---

## PR Checklist

Before submitting a PR, verify:

- [ ] CHANGELOG.md updated with changes
- [ ] CURRENT_STATUS.md reflects new progress
- [ ] Relevant phase README updated (if feature/behavior changed)
- [ ] Code comments added for public functions
- [ ] README.md updated (if user-facing change)
- [ ] Architecture docs updated (if system design changed)

---

## Release Documentation

When releasing a new version:

1. Move `[Unreleased]` entries in CHANGELOG to new version section
2. Update version in README badges (if any)
3. Update CURRENT_STATUS.md to reflect completed phase
4. Tag the release in git: `git tag v0.2.0`
