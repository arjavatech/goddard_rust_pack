# Tap-Time Working-Hours Integration Plan

## Goal

Integrate Goddard employee management with an existing Tap-Time company. Goddard manages employee profile updates and PIN-reset UI; Tap-Time remains the source of truth for clock-ins, clock-outs, hours, scheduled reports, and report settings.

Tap-Time must remain reusable by other applications. Its new integration layer therefore uses generic `integration_*` names and `/integrations/v1/*` routes only; it does not contain Goddard-specific names.

## Boundaries and compatibility

- One Goddard school links to one **existing** Tap-Time company. Goddard never creates Tap-Time companies.
- Existing Tap-Time UI and routes remain in place. Generic partner APIs, mapping tables, report identifiers, and audits are additive.
- Tap-Time currently uses plaintext employee PINs. This project keeps that behavior to avoid changing the existing Tap-Time application. Goddard accepts exactly four numeric PIN digits and must never store, log, cache, or return a PIN.
- Goddard Web calls only Goddard Backend. Goddard Backend calls Tap-Time over a protected service-to-service channel. No Tap-Time secret reaches a browser.

## Identity and employee reconciliation

After a school is connected, both systems retain a permanent mapping:

```text
Goddard employee.id <-> Tap-Time employee.emp_id
```

For first-time reconciliation only, match employees by normalized phone number (`+15551234567`). Do not match by name or email. An exact one-to-one phone match is merely proposed; a Super Admin must confirm it. Missing or duplicate phone numbers require manual mapping. Once stored, permanent IDs—not phone/email/name—are used for all updates.

New connected-school employees are created in Goddard first, then idempotently created in Tap-Time through an employee-sync outbox. CSV upload continues to exclude PINs; imported staff have `pending_pin` status until an admin or the employee sets one.

## Security architecture

```text
Goddard Web -> Goddard Backend -> Tap-Time /integrations/v1 -> Tap-Time PostgreSQL
```

- Goddard Backend signs a service JWT for each Tap-Time request (issuer, audience, client ID, tenant ID, scopes, 60-second expiry, key ID, and unique `jti`).
- Tap-Time validates the registered client public key, issuer/audience, expiry, scope, active tenant mapping, one-time nonce, and idempotency key.
- All calls use HTTPS, request IDs, rate limits, and redacted structured logging.
- Existing Tap-Time shared mobile API keys must not access generic integration routes.
- Tap-Time registers each client public signing key **and trusted issuer**. A token with a valid signature but an unregistered issuer is rejected.
- Super Admin alone links/disconnects schools and resolves reconciliation conflicts. School Admin/Super Admin manage school reports. Employees can reset only their own PIN and see only their own reports.

## Tap-Time schema

Create these additive tables:

| Table | Key fields | Purpose |
|---|---|---|
| `integration_clients` | `client_key`, public keys, scopes, active | Trusted external applications |
| `integration_connection_codes` | client, company, code hash, expiry, redeemed state | One-time existing-company link codes |
| `integration_tenant_links` | client, external tenant ID, company, status | External tenant to Tap-Time company mapping |
| `integration_entity_links` | client, company, entity type, external/internal IDs | Permanent employee mappings |
| `integration_request_nonces` | client, `jti`, expiry | JWT replay prevention |
| `integration_audit_events` | client, company, actor, action, request ID | Auditable external writes |

Also add stable UUID `id`, update/delete metadata, and soft-delete support to `daily_report_table`. Add stable UUID IDs to `company_report_type` while keeping legacy routes compatible.

## Generic Tap-Time APIs

All below are protected by the service JWT and validate that the external tenant has an active link to the requested company.

```text
POST   /integrations/connection-codes
GET    /integrations/connection-codes
DELETE /integrations/connection-codes/:code_id

POST   /integrations/v1/connections/redeem
GET    /integrations/v1/tenants/:external_tenant_id/connection
GET    /integrations/v1/tenants/:external_tenant_id/connection/health
DELETE /integrations/v1/tenants/:external_tenant_id/connection

PUT    /integrations/v1/tenants/:external_tenant_id/employees/:external_employee_id
POST   /integrations/v1/tenants/:external_tenant_id/employees/:external_employee_id/pin
POST   /integrations/v1/tenants/:external_tenant_id/employees/:external_employee_id/deactivate
GET    /integrations/v1/tenants/:external_tenant_id/employees

GET    /integrations/v1/tenants/:external_tenant_id/reports/daily
GET    /integrations/v1/tenants/:external_tenant_id/reports/date-range
GET    /integrations/v1/tenants/:external_tenant_id/reports/salaried
GET    /integrations/v1/tenants/:external_tenant_id/reports/pending-checkout
PATCH  /integrations/v1/tenants/:external_tenant_id/reports/:report_id
DELETE /integrations/v1/tenants/:external_tenant_id/reports/:report_id

GET    /integrations/v1/tenants/:external_tenant_id/report-settings
POST   /integrations/v1/tenants/:external_tenant_id/report-settings
PATCH  /integrations/v1/tenants/:external_tenant_id/report-settings/:setting_id
DELETE /integrations/v1/tenants/:external_tenant_id/report-settings/:setting_id
```

Employee upsert fields are first name, last name, phone number, optional email, active state, and external actor ID. PIN updates are separate and immediate; PINs are never queued. Reports return stable report IDs, employee mapping IDs, dates, check-in/out, calculated hours, type, and pending-checkout state. Tap-Time computes all work-hour and salaried totals using its company-level default report type.

## Goddard schema

Create:

| Table | Purpose |
|---|---|
| `tap_time_connections` | school-to-company link, safe company metadata, status, health, connector audit |
| `tap_time_employee_links` | Goddard employee to Tap-Time employee mapping and sync state |
| `tap_time_sync_outbox` | retryable non-sensitive employee profile/deactivation events |
| `tap_time_audit_events` | Goddard-side user/action/request audit trail |

`tap_time_sync_outbox` may contain only safe profile fields. PIN values are never written to it.

## Goddard APIs

### Super Admin

```text
POST   /tap-time/connections
GET    /tap-time/connections/:school_id
GET    /tap-time/connections/:school_id/health
DELETE /tap-time/connections/:school_id
POST   /tap-time/connections/:school_id/reconcile
POST   /tap-time/connections/:school_id/retry-sync
```

### Employee lifecycle and PIN

Secure existing employee list/detail/create/update/activate/deactivate/bulk routes with school authorization, add sync status to responses, and add:

```text
POST /employees/:employee_id/tap-time-pin
GET  /employees/:employee_id/tap-time-sync
POST /employees/:employee_id/tap-time-sync/retry

POST /me/time-attendance/pin
GET  /me/time-attendance/daily?date=YYYY-MM-DD
GET  /me/time-attendance/salaried?anchor_date=YYYY-MM-DD
```

Employee self PIN reset requires recent Goddard reauthentication. Self-service reports are read-only and use only the authenticated employee mapping. The salaried period is the linked company school default, not employee-selectable.

### Admin reports and settings

```text
GET    /time-attendance/daily
GET    /time-attendance/date-range
GET    /time-attendance/salaried
GET    /time-attendance/pending-checkout
PATCH  /time-attendance/reports/:report_id
DELETE /time-attendance/reports/:report_id

GET    /time-attendance/report-settings
POST   /time-attendance/report-settings
PATCH  /time-attendance/report-settings/:setting_id
DELETE /time-attendance/report-settings/:setting_id
```

Report edit/delete requires an audit reason. Tap-Time performs the actual write and soft deletion.

## UI

### Super Admin

Add a Tap-Time Integration page to link an existing company with a one-time code, check health, disconnect, reconcile employees by phone, manually resolve conflicts, and retry failures.

### Admin

Enhance Employee Management with connection/sync status, last error, retry, and PIN set/reset. Add a Time & Attendance section with Report Summary (Overview, Daily, Date Range, Salary hours, Day Trends, Two-Day Report, and Pending Checkouts) and Report Settings. Check-in/check-out photos are deliberately excluded because Tap-Time does not persist them. Reuse Goddard filters, pagination, table/grid, date controls, exports, and confirmation modal patterns.

### Employee

Enhance Employee Dashboard with My Daily Hours, My Salaried Report, and Reset Clock-in PIN. Employees have no report edit/delete access.

## Implementation phases

1. Create generic Tap-Time schemas, opaque tenant-token middleware, nonce/idempotency/audit support, and generic API contracts.
2. Add connection-code and tenant-link APIs, then Super Admin Goddard connection storage/APIs.
3. Add employee mapping/upsert/deactivate APIs and Goddard sync outbox/worker.
4. Add phone normalization/reconciliation workflow and employee PIN handling.
5. Add generic report/report-settings APIs and Goddard backend proxies.
6. Build Super Admin, Admin, and Employee Goddard UI screens.
7. Test tenant isolation, replay protection, link-code expiry, phone-match conflicts, sync retries, PIN redaction, personal-report authorization, report mutations, exports, and legacy Tap-Time compatibility.
8. Pilot one school, reconcile employees, set PINs, validate report totals, then enable further schools.

## Required deployment configuration

Goddard Backend reads these internal values at server startup. They belong in an ignored environment file and never in the web UI or source control. Users do not create or paste these values: they only generate and redeem the one-time connection code.

```text
TAP_TIME_API_URL=https://<tap-time-api>
TAP_TIME_CONNECTION_ENCRYPTION_KEY=<base64-encoded 32-byte server key>
```

Tap-Time hashes the tenant-scoped connection token; Goddard encrypts its copy using the server key. No PEM files, client IDs, key IDs, or signing keys are part of setup.
