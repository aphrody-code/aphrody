# Google · Cloud / GCP APIs

Python tooling for Google Cloud Platform services: SRE/SLO management, security scanning, BigQuery integrations, Workspace/Gmail utilities, Kubernetes, Spanner ORM, and Ads API adapters.

> Part of [`docs/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 24 repos (12 active / 12 archived).

## SRE / SLO Tooling

### [slo-generator](https://github.com/google/slo-generator)
**★ 561 · `active` · pushed 2026-04 · Apache-2.0**

Computes Service Level Indicators, Service Level Objectives, Error Budgets, and Burn Rates against pluggable backends (Prometheus, Datadog, Cloud Monitoring, Elasticsearch, and others), then exports reports to supported targets. Configuration is YAML/JSON-driven; v2 supports deployment on Cloud Run, Kubernetes, and Cloud Build pipelines. Published on PyPI as `slo-generator`.

---

## GCP Security / Scanning

### [gcp_scanner](https://github.com/google/gcp_scanner)
**★ 357 · `active` · pushed 2025-12 · Apache-2.0**  
Topics: `gcp` `google-cloud-platform` `scanning-tool` `security` `automation`

A GCP resource scanner that determines what level of access a set of credentials holds across a GCP project. Supports credential extraction from VM instance metadata, gcloud profiles, OAuth2 refresh tokens, and JSON service account keys. Covers GCE, GCS, GKE, Cloud SQL, BigQuery, Spanner, Pub/Sub, Cloud Functions, Bigtable, KMS, and more. Runs without the gcloud SDK installed; Linux-only execution model.

---

## OAuth / Authentication

### [gmail-oauth2-tools](https://github.com/google/gmail-oauth2-tools)
**★ 478 · `active` · pushed 2025-07 · Apache-2.0**

Reference tools and sample code for authenticating mail clients to Gmail using OAuth2 (XOAUTH2 SASL mechanism). Contains scripts for generating OAuth2 tokens, refreshing them, and encoding them for IMAP/SMTP usage. Useful for integrating legacy or custom mail clients with Google accounts.

---

## Workspace / Gmail Utilities

### [import-mailbox-to-gmail](https://github.com/google/import-mailbox-to-gmail)
**★ 353 · `active` · pushed 2026-02 · Apache-2.0**

Imports `.mbox` files into Google Workspace (formerly G Suite) Gmail accounts via the Gmail API. Supports batch import for Workspace migrations; useful for organizations moving from on-premises mail servers to Google Workspace. Maintained as a practical migration utility rather than a library.

---

## ML / Research Workflow on Cloud

### [caliban](https://github.com/google/caliban)
**★ 504 · `archived` · pushed 2024-06 · Apache-2.0**  
Topics: `ai-platform` `docker` `google-cloud` `python3` `research-tool`

CLI tool for running ML research workflows locally (via Docker) and on Google AI Platform (Vertex AI predecessor). Manages experiment configuration, Docker image building, and job submission without requiring manual cloud SDK scripting. Archived; Vertex AI Pipelines is the successor path.

---

## BigQuery

### [encrypted-bigquery-client](https://github.com/google/encrypted-bigquery-client)
**★ 175 · `archived` · pushed 2018-02 · Apache-2.0**

Experimental client-side encryption layer for BigQuery. Allowed encrypting field values before insertion so that the BigQuery service itself never received plaintext. Archived; predates current BigQuery CMEK and Confidential Computing offerings.

### [sa360-bigquery-bootstrapper](https://github.com/google/sa360-bigquery-bootstrapper)
**★ 6 · `active` · pushed 2025-05 · Apache-2.0**

Scripts for bootstrapping Search Ads 360 data exports into BigQuery, including schema setup and initial data transfer configuration. Utility-class tooling for Ads reporting pipelines.

---

## Firebase

### [csv-to-firestore](https://github.com/google/csv-to-firestore)
**★ 27 · `active` · pushed 2026-04 · Apache-2.0**

Utility for uploading CSV data directly into Firestore collections. Handles field type inference and batch writes. Actively maintained as a practical ETL helper for Firestore-backed applications.

---

## Kubernetes / GKE

### [kasane](https://github.com/google/kasane)
**★ 172 · `archived` · pushed 2021-08 · Apache-2.0**

A simple Kubernetes deployment manager that layers YAML patches (using jsonnet or plain YAML) over base manifests. Aimed at reducing the complexity of Helm for simpler deployment scenarios. Archived; superseded by Kustomize and Helm v3 adoption.

### [gke-cloud-dns-tls](https://github.com/google/gke-cloud-dns-tls)
**★ 14 · `archived` · pushed 2023-04 · Apache-2.0**  
Topics: `gke` `cloud-dns` `apigee`

Automation scripts and configuration for integrating GKE workloads with Cloud DNS and provisioning TLS certificates. Demonstrates patterns for external DNS management and certificate issuance in GKE clusters. Archived.

---

## Cloud Database (Spanner)

### [python-spanner-orm](https://github.com/google/python-spanner-orm)
**★ 40 · `active` · pushed 2025-08 · Apache-2.0**

ORM for Cloud Spanner written in Python. Provides a model-based interface for defining schemas, running queries, and managing migrations against Spanner instances. Actively maintained; lighter-weight alternative to using the official Spanner client library directly.

---

## Workspace Automation

### [create-service-account](https://github.com/google/create-service-account)
**★ 28 · `active` · pushed 2026-01 · Apache-2.0**

Scripts for automating Google Workspace service account creation in the context of migration products (Migrate for Workspace). Handles IAM role assignments and key generation for migration scenarios.

---

## Vertex AI / Dataflow

### [dataflow-ml-starter](https://github.com/google/dataflow-ml-starter)
**★ 24 · `active` · pushed 2026-02 · Apache-2.0**

Starter template for building ML inference pipelines on Cloud Dataflow (Apache Beam). Provides boilerplate for model loading, batch prediction, and pipeline configuration targeting Vertex AI and Dataflow runners.

### [vertex-pipelines-boilerplate](https://github.com/google/vertex-pipelines-boilerplate)
**★ 10 · `archived` · pushed 2024-07 · Apache-2.0**

Boilerplate for setting up Kubeflow-based pipelines on Vertex AI Pipelines. Includes component definitions, pipeline compilation, and submission scripts. Archived.

---

## Cloud Testing Infrastructure

### [citest](https://github.com/google/citest)
**★ 60 · `archived` · pushed 2021-01 · Apache-2.0**

Python library for writing integration tests against cloud services. Provides abstractions for agent-based test patterns (send a command, observe the result in the cloud). Was used internally for Spinnaker integration testing. Archived.

---

## GCP Orchestration

### [symphony-gcp](https://github.com/google/symphony-gcp)
**★ 4 · `active` · pushed 2026-05 · Apache-2.0**

Recently created (2025-03) active repository in the GCP automation space. No description or topics currently published; under active development as of 2026-05.

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [analytics-settings-database](https://github.com/google/analytics-settings-database) | 58 | archived | Google Analytics settings backup/restore tooling |
| [alligator2](https://github.com/google/alligator2) | 43 | archived | Sample: Google My Business API + Cloud Natural Language API |
| [hotel-ads-etl-tool](https://github.com/google/hotel-ads-etl-tool) | 10 | archived | ETL from Hotel Ads API to BigQuery |
| [coop-analytics](https://github.com/google/coop-analytics) | 10 | archived | Analytics tooling (no description) |
| [appengine_xblock_runtime](https://github.com/google/appengine_xblock_runtime) | 19 | archived | App Engine XBlock runtime for OpenEdX |
| [python-cloud-utils](https://github.com/google/python-cloud-utils) | 23 | archived | Miscellaneous GCP Python utilities |
| [cloud-berg](https://github.com/google/cloud-berg) | 26 | archived | Run GPU-backed experiments on gcloud |
| [cloudprint_logocert](https://github.com/google/cloudprint_logocert) | 27 | archived | Google Cloud Print logo certification automation |
| [campaign-manager-bulk-uploader](https://github.com/google/campaign-manager-bulk-uploader) | 10 | archived | Campaign Manager 360 bulk trafficking via Python/Angular |
