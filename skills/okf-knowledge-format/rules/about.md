# OKF notes

- `type` is the single point of required structure. Identifying/routing metadata → frontmatter; explanation and evidence → body.
- OKF references domain schemas (Avro, Protobuf, OpenAPI) rather than replacing them — link out, don't inline a competing schema.
- **living-docs vs. OKF:** living-docs governs a repo's *internal* decision/requirement records (ADR/PRD/BDR); OKF is the portable, exchange-oriented *format* for knowledge bundles meant to be shared or agent-consumed. They compose — a living-docs research corpus can be authored as a spec-conformant OKF bundle.
