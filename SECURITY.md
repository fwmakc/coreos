# Security Policy

## Supported Versions

| Version   | Supported          |
|-----------|--------------------|
| < 0.1.0   | :x: Pre-alpha      |
| 0.1.0+    | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Workspace, please report it responsibly.

**Please do NOT open a public issue for security bugs.**

Instead, send an email to: **security@Workspace.dev**

Include the following details:
- Description of the vulnerability
- Steps to reproduce (if applicable)
- Potential impact
- Suggested fix (if any)

We aim to respond within 72 hours and will keep you informed throughout the remediation process.

## Security Design Principles

- **Capability-based security:** All resource access is mediated through capability tokens.
- **Zero-trust P2P:** All mesh traffic is encrypted with WireGuard; no central server holds keys.
- **Sandboxed apps:** Third-party apps run inside V8 isolates with no direct host access.
- **Data sovereignty:** User data never leaves the device without explicit, revocable consent.

## Audit Log

Security fixes will be documented in the changelog and attributed to the reporter (with permission).
