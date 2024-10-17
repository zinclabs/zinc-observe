<p align="center">
  <a href="https://openobserve.ai"><img src="https://openobserve.ai/img/logo/logo_horizontal.svg" alt="OpenObserve"></a>
</p>
<p align="center">
    <em>🚀 10x easier, 🚀 140x lower storage cost, 🚀 high performance, 🚀 petabyte scale - Elasticsearch/Splunk/Datadog alternative for 🚀 (logs, metrics, traces).</em>
</p>
<p align="center">
<a href="https://github.com/openobserve/openobserve" target="_blank">
    <img src="https://img.shields.io/github/last-commit/openobserve/openobserve" alt="Last Commit">
</a>
<a href="https://github.com/openobserve/openobserve/stargazers" target="_blank">
    <img src="https://img.shields.io/github/stars/openobserve/openobserve" alt="GitHub Stars">
</a>
<a href="https://github.com/openobserve/openobserve/issues" target="_blank">
    <img src="https://img.shields.io/github/issues/openobserve/openobserve" alt="GitHub Issues">
</a>
<a href="https://github.com/openobserve/openobserve/graphs/contributors" target="_blank">
    <img src="https://img.shields.io/github/contributors/openobserve/openobserve" alt="Contributors">
</a>
<a href="https://github.com/openobserve/openobserve/releases" target="_blank">
    <img src="https://img.shields.io/github/v/release/openobserve/openobserve" alt="GitHub Release">
</a>
</p>

OpenObserve (O2 for short) is a cloud-native observability platform built specifically for logs, metrics, traces, analytics, RUM (Real User Monitoring - Performance, Errors, Session Replay) designed to work at petabyte scale.

It is straightforward and easy to operate, in contrast to Elasticsearch, which requires understanding and tuning numerous settings. Get OpenObserve up and running in under 2 minutes.

OpenObserve serves as a seamless replacement for Elasticsearch for users who ingest data using APIs and perform searches. OpenObserve comes with its own user interface, eliminating the need for separate installation.

You can reduce your log storage costs by ~140x compared to Elasticsearch by using OpenObserve. Below, we present the results from pushing logs from our production Kubernetes cluster to both Elasticsearch and OpenObserve using Fluent Bit.

![OpenObserve Vs Elasticsearch](./screenshots/zo_vs_es.png)

## 🎥 Introduction Video

https://github.com/openobserve/openobserve/assets/4242188/77c71e8c-23f6-4123-b42a-7113b464f7a6

## 🌟 Features:

- **Logs, Metrics, Traces**: Comprehensive support for various data types.
- **OpenTelemetry Support**: Full compatibility with OTLP for logs, metrics, and traces.
- **Real User Monitoring (RUM)**: Includes performance tracking, error logging, and session replay.
- **Alerts & Dashboards**: Features over 18 different chart types for comprehensive data visualization.
- **Advanced Ingest and Query Functions**: Aid in enrichment, redaction, log reduction, and compliance, like redacting sensitive data from logs.
- **Advanced Embedded GUI**: Intuitive and user-friendly interface.
- **SQL and PromQL Support**: Query logs and traces with SQL, and metrics with SQL and PromQL.
- **Single Binary Installation**: Easy installation and running, with binaries available for multiple platforms under [releases](https://github.com/openobserve/openobserve/releases).
- **Versatile Storage Options**: Supports local disk, S3, MinIO, GCS, Azure Blob Storage.
- **High Availability and Clustering**: Ensures reliable and scalable performance.
- **Dynamic Schema**: Adapts to your data structure seamlessly.
- **Built-in Authentication**: Secure and ready to use.
- **Ease of Operation**: Designed for simplicity and efficiency.
- **Seamless Upgrades**: Hassle-free updates.
- **Multilingual UI**: Supports 11 languages, including English, Spanish, German, French, Chinese, and more.

For a full list of features, check the [documentation](https://openobserve.ai/docs/#project-status-features-and-roadmap).

## ⚡️ Quick start

### 🐳 Docker:
```bash
docker run -d \
      --name openobserve \
      -v $PWD/data:/data \
      -p 5080:5080 \
      -e ZO_ROOT_USER_EMAIL="root@example.com" \
      -e ZO_ROOT_USER_PASSWORD="Complexpass#123" \
      public.ecr.aws/zinclabs/openobserve:latest
```

### 🐙 Docker Compose:
```yaml
services:
  openobserve:
    image: public.ecr.aws/zinclabs/openobserve:latest
    restart: unless-stopped
    environment:
      ZO_ROOT_USER_EMAIL: "root@example.com"
      ZO_ROOT_USER_PASSWORD: "Complexpass#123"
    ports:
      - "5080:5080"
    volumes:
      - data:/data
volumes:
  data:
```

For other ways to quickly install OpenObserve or use OpenObserve cloud, check [quickstart documentation](https://openobserve.ai/docs/quickstart).

For installing OpenObserve in HA mode, check [HA deployment documentation](https://openobserve.ai/docs/ha_deployment/).

## Enterprise Vs Open source Vs Cloud edition

OpenObserve is available in three different editions:


| Feature | Open Source (Self hosted) | Enterprise (Self hosted) | Cloud |
| --- | --- | --- | --- | 
| Logs | ✅ | ✅ | ✅ |
| Metrics | ✅ | ✅ | ✅ |
| Traces | ✅ | ✅ | ✅ |
| RUM | ✅ | ✅ | ✅ |
| Alerts | ✅ | ✅ | ✅ |
| Dashboards | ✅ | ✅ | ✅ |
| Reports | ✅ | ✅ | ✅ |
| VRL functions | ✅ | ✅ | ✅ |
| Pipelines | ✅ | ✅ | ✅ |
| High Availability | ✅ | ✅ | ✅ |
| Multitenancy (Organizations) | ✅ | ✅ | ✅ |
| Dynamic schema and schema evolution | ✅ | ✅ | ✅ |
| Advanced multilingual GUI | ✅ | ✅ | ✅ |
| Single Sign On | ❌ | ✅ | ✅ |
| Role Based Access Control (RBAC) | ❌ | ✅ | ✅ |
| Federated search / Super cluster | ❌ | ✅ | ❌ |
| Query management | ❌ | ✅ | ❌ |
| Workload management (QoS) | ❌ | ✅ | ❌ |
| Audit trail | ❌ | ✅ | ❌ |
| Ability to influence roadmap | ❌ | ✅ | ✅ on enterprise plan |
| License | AGPL | Enterprise | Cloud |
| Support | Community | Enterprise | Cloud |
| Cost | Free | If self hosted, free for up to 200 GB/Day data ingested <br> Paid thereafter  | Free 200 GB/Month data ingested <br> Paid thereafter |


## 📷 Screenshots

### Home

![Home](./screenshots/zo_home.png)

### Logs

![Logs](./screenshots/logs.png)

### Traces (OpenTelemetry)

Trace details page
![Traces using OpenTelemetry](./screenshots/traces.png)

Golden metrics based on traces
![Traces golden metrics](./screenshots/traces-overall.png)

### Visualizations and Dashboards

![Dashboard](./screenshots/dashboard.png)
![Dashboard](./screenshots/dashboard2.png)
![Create panel](./screenshots/create-panel.png)
![Map](./screenshots/map.png)

### Front end monitoring

Performance analytics
![Performance](./screenshots/performance.png)

Session replay
![Session replay](./screenshots/session-replay.png)

Error tracking
![Error tracking](./screenshots/error-tracking.png)


### Alerts

![Alerts](./screenshots/alerts.png)


### Streams

![Streams](./screenshots/streams.png)

### Ingestion

![Ingestion](./screenshots/ingestion1.png)

![Ingestion](./screenshots/ingestion2.png)

### SBOM

Software Bill of Materials for OpenObserve

#### Rust

SBOM can be found [here](./openobserve.cdx.xml). You can analyze it using [dependency track](https://dependencytrack.org/).

In order to generate the SBOM, you can use the following commands:

Install cargo-cyclonedx:

````bash
cargo install cargo-cyclonedx
````

Generate the SBOM:
```bash
cargo-cyclonedx cyclonedx
```

#### JavaScript

SBOM can be found [here](./web/sbom.json ). You can analyze it using [dependency track](https://dependencytrack.org/).

In order to generate the SBOM, you can use the following commands:

Install cyclonedx-npm:

````bash
npm install --global @cyclonedx/cyclonedx-npm
````

Generate the SBOM:
```bash
cd web
cyclonedx-npm > sbom.json         
```


## ⚖️ License

OpenObserve is licensed under the AGPL-3.0 license. For more details, see the [LICENSE](https://github.com/openobserve/openobserve/blob/main/LICENSE).

## 🌍 Community

### 🔗 Join our Slack Channel

[![Slack](./screenshots/slack.png)](https://short.openobserve.ai/community)

Easiest way to get support is to join the [Slack channel](https://short.openobserve.ai/community).

### 📱 Join our WeChat Group

<img src="./screenshots/wechat_qr.jpg" width="300">
