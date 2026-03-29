# Changelog

All notable changes to SolUPG will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Initial project structure and documentation
- Phase 1-6 development roadmap documentation
- Architecture overview documentation
- Apache 2.0 license
- Contributing guidelines
- **Documentation System**
  - `CHANGELOG.md` — Version tracking
  - `docs/DOCUMENTATION_GUIDE.md` — Documentation maintenance guide
  - `docs/development/CURRENT_STATUS.md` — Live development status
  - `.github/PULL_REQUEST_TEMPLATE.md` — PR template with doc checklist
  - `.github/ISSUE_TEMPLATE/bug_report.md` — Bug report template
  - `.github/ISSUE_TEMPLATE/feature_request.md` — Feature request template
- Updated `CONTRIBUTING.md` with mandatory documentation requirements
- Updated `README.md` with documentation links

### Changed
- Renamed project from UPG to SolUPG (Solana Universal Payment Gateway)

---

## [0.1.0] - 2026-03-29

### Added
- **Project Setup**
  - Initial repository structure
  - README with project overview, architecture diagram, and roadmap
  - Documentation structure for all 6 development phases
  - Folder structure for programs, services, SDK, and tests

- **Documentation**
  - `docs/architecture/overview.md` — System architecture deep-dive
  - `docs/phase-1-onchain-programs/` — On-chain program specifications
  - `docs/phase-2-routing-engine/` — Routing engine & directory service specs
  - `docs/phase-3-api-gateway/` — API gateway & SDK specs
  - `docs/phase-4-clearing-reconciliation/` — Clearing & reconciliation specs
  - `docs/phase-5-compliance-monitoring/` — Compliance & monitoring specs
  - `docs/phase-6-testing-deployment/` — Testing & deployment specs

- **Open Source**
  - Apache 2.0 License
  - CONTRIBUTING.md with code standards and PR process
  - .gitignore for Rust, Node.js, and Anchor projects

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | 2026-03-29 | Initial project setup with documentation |

---

## Upcoming

### Phase 1 (Next)
- [ ] `solupg-payment` — Core payment program
- [ ] `solupg-escrow` — Escrow program
- [ ] `solupg-splitter` — Fee splitting program
- [ ] `solupg-swap` — Token swap integration

See [Phase 1 Documentation](./docs/phase-1-onchain-programs/README.md) for details.
