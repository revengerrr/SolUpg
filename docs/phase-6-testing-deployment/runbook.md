# SolUPG Incident Response Runbook

> **Audience**: On-call engineers, platform operators
> **Companion**: [`threat-model.md`](../security/threat-model.md)
> **Status**: Phase 6 baseline

---

## 1. Severity Tiers

| Sev | Definition | Examples | Response time | Who responds |
|-----|------------|----------|---------------|--------------|
| **SEV-1** | Funds at risk, active exploit, or full outage | Program exploit, DB breach, >50% error rate | **< 15 min** | On-call + eng lead + security lead |
| **SEV-2** | Degraded service affecting multiple merchants | Elevated errors, single-service down, RPC flapping | **< 1 hr** | On-call |
| **SEV-3** | Minor degradation, workaround exists | Dashboard lag, slow reconciliation | **< 4 hr (business)** | On-call next business day |
| **SEV-4** | Cosmetic or informational | Typo in error message | **Backlog** | Normal triage |

## 2. On-Call Roster Template

- Primary on-call: rotates weekly (Mon 09:00 → Mon 09:00 local).
- Secondary on-call: covers if primary unreachable within 5 min.
- Security escalation: security lead (24/7 for SEV-1 involving funds).
- Comms lead: for SEV-1 only — handles merchant and public comms.

Populate the actual roster in PagerDuty / Opsgenie; this doc only defines roles.

## 3. First 15 Minutes (SEV-1 Playbook)

1. **Acknowledge page** in PagerDuty / Slack.
2. **Open incident channel** `#incident-YYYYMMDD-<short-desc>`.
3. **Post initial status** in the channel:
   - What is alerting?
   - Blast radius (which merchants / volume affected)?
   - Current error rate / impacted endpoints.
4. **Declare severity** and page additional responders if SEV-1.
5. **Start incident log** — pin a message and update every status change with timestamp.
6. **Decide: stabilize vs. investigate.** Stabilizing actions include:
   - Rate-limit or block specific API keys / IPs (api-gateway config).
   - Pause specific route types in routing-engine (feature flag).
   - Scale up replicas.
   - Fail over to backup RPC.
   - **DO NOT** freeze the on-chain program unless SEV-1 with fund risk (requires security lead + eng lead approval).

## 4. Playbooks

### 4.1 Program Halt / Rollback

> Used when a bug in a SolUPG program is actively losing funds.

**Precondition**: upgrade authority is available (hardware wallet online).

1. Convene: eng lead + security lead + 1 witness.
2. Verify the issue reproduces on devnet.
3. Build the fixed program from a hot-patch branch.
4. Generate the upgrade transaction:
   ```bash
   anchor build
   solana program deploy \
     --program-id <PROGRAM_ID> \
     --keypair <UPGRADE_AUTHORITY> \
     --url mainnet-beta \
     target/deploy/solupg_<program>.so
   ```
5. Verify on-chain hash matches local artifact.
6. Announce in status channel and merchant comms.
7. Post-incident: retain old `.so` for forensic analysis.

> **If upgrade authority is frozen**: escalate to governance; document the issue publicly while remediation is prepared.

### 4.2 RPC Outage

Symptoms: routing-engine errors spike, `/health` returns non-200 from upstream checks, indexer lag.

1. Check multi-RPC dashboard (Grafana: `solupg-rpc-health`).
2. Rotate routing-engine's `SOLANA_RPC_URL` to backup provider:
   ```bash
   kubectl -n solupg set env deployment/routing-engine \
     SOLANA_RPC_URL=https://backup-rpc.example.com
   kubectl -n solupg rollout restart deployment/routing-engine
   ```
3. Reduce `compute_unit_price` cap if network congestion is the cause.
4. Notify merchants via status page if >5 min of degraded service.
5. Post-incident: file an RFE to add this provider to the quorum set.

### 4.3 Database Failover

Symptoms: `sqlx` errors in all services, healthchecks failing, Postgres CPU at 100% or replication lag alert.

1. Check replication lag from primary → replica (`pg_stat_replication`).
2. If primary is unreachable, promote replica:
   ```bash
   # Managed: cloud provider failover button.
   # Self-managed:
   pg_ctl promote -D /var/lib/postgresql/data
   ```
3. Update services' `DATABASE_URL` to point to new primary.
4. Restart all services in order: directory-service → clearing-engine → monitoring → routing-engine → api-gateway.
5. Verify no reconciliation runs are mid-flight.
6. Post-incident: confirm WAL archive is intact for the window around the failover.

### 4.4 Key Compromise

Scope: any production key (upgrade authority, DB password, API key signing secret, webhook secret, RPC API keys).

1. **Immediately**: revoke the compromised credential at the source.
2. Rotate the credential (new key / password / secret).
3. Update secret manager and trigger service rollouts.
4. Audit `audit_log` for actions taken with the compromised credential during the exposure window.
5. File a post-mortem with scope of exposure.
6. If upgrade authority: initiate emergency rotation ceremony (multi-person, hardware wallet).

### 4.5 Fraud Spike / Sanctions Hit

Symptoms: Monitoring service alerts on velocity / threshold rules, sanctions screening returns positive hits.

1. Review `monitoring` dashboard for the triggering merchants/wallets.
2. For confirmed sanctions hits: block the wallet in `sanctions_list` (immediate effect), notify compliance lead.
3. For fraud rule hits: review transactions, temporarily freeze merchant if warranted (via admin endpoint).
4. Log all actions in `audit_log` with reason.
5. Regulatory reporting: follow jurisdiction-specific SARs process.

### 4.6 Webhook Delivery Storm

Symptoms: Merchant-side webhook endpoints returning 5xx at scale, queue backlog growing.

1. Identify the affected merchant(s) via `/v1/webhooks/deliveries`.
2. Pause deliveries to those merchants:
   ```bash
   curl -X POST https://api.solupg.io/admin/webhooks/pause \
     -H "X-Admin-Token: $ADMIN_TOKEN" \
     -d '{"merchant_id": "..."}'
   ```
3. Contact merchant via pre-defined channel.
4. Resume delivery after merchant confirms their endpoint is healthy; backlog drains over next few minutes.

## 5. Communication Templates

### 5.1 Initial Incident Announcement (Status Page)

```
[Investigating] We are currently investigating elevated error rates
on the SolUPG API. A subset of payment requests may fail or be delayed.
Funds are safe. Next update in 15 minutes.
```

### 5.2 Mid-Incident Update

```
[Identified] We have identified the root cause as <brief cause>.
Mitigation is in progress. Estimated time to restore: <X minutes>.
Funds remain safe. Next update in 15 minutes.
```

### 5.3 Resolution

```
[Resolved] The incident affecting <component> from <start> to <end>
has been resolved. <one-line summary of fix>. Full post-mortem will
be published within 5 business days.
```

### 5.4 Merchant Direct Notification (for SEV-1 touching their account)

```
Subject: [SolUPG] Service incident affecting your account

Hi <merchant>,

Between <start UTC> and <end UTC> our platform experienced <issue>.
During this window, <specific impact on their account>. We have
<remediation taken>. No action is required from you.

Your funds and balances were not affected. If you observe any
unexpected behaviour please reply to this email.

We will publish a detailed post-mortem within 5 business days.

— The SolUPG team
```

## 6. Post-Incident Process

Within 5 business days of resolution:

- [ ] Post-mortem document in `docs/incidents/YYYY-MM-DD-<slug>.md`.
- [ ] Sections: Summary, Timeline, Root Cause, Impact, What Went Well, What Went Wrong, Action Items.
- [ ] File GitHub issues for all action items with owners and due dates.
- [ ] Update this runbook if a new playbook is needed.
- [ ] Update threat model if a new attack surface was revealed.
- [ ] Share summary with merchants if they were directly affected.

## 7. Contact Directory (template — populate before launch)

| Role | Name | Primary | Backup |
|------|------|---------|--------|
| Eng lead | TBD | TBD | TBD |
| Security lead | TBD | TBD | TBD |
| On-call primary | rotation | PagerDuty | — |
| Comms lead | TBD | TBD | TBD |
| Legal / compliance | TBD | TBD | TBD |
| Cloud provider support | TBD | 24/7 hotline | — |
| RPC provider primary | TBD | TBD | — |
| RPC provider backup | TBD | TBD | — |

## 8. Dry-Run Cadence

- **Monthly**: paper drill of one playbook (rotate).
- **Quarterly**: live drill in staging environment.
- **Post-incident**: re-run the relevant playbook on a non-prod system within 2 weeks.

## 9. Tools

| Tool | Purpose | Link |
|------|---------|------|
| PagerDuty | On-call, paging | TBD |
| Grafana | Metrics dashboards | TBD |
| Prometheus | Metrics source | TBD |
| Loki/ELK | Log aggregation | TBD |
| Status page | Public incident comms | TBD |
| GitHub | Incident tracking, post-mortems | TBD |
| Slack | Internal coordination | TBD |
