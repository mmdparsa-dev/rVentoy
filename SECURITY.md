# Security Policy

Due to the nature of **rVentoy**, this software requires elevated administrative privileges to perform raw read and write operations on physical storage devices. We take security, memory safety, and disk data integrity very seriously.

---

## Supported Versions

Only the latest release and the current `main` branch receive security patches and updates.

| Version | Supported |
| --- | --- |
| `main` | :white_check_mark: |
| Latest | :white_check_mark: |
| Older | :x: |

---

## Reporting a Vulnerability

If you discover a security vulnerability—especially issues related to:

* Erroneous physical drive selection or target resolution
* Privilege escalation vectors
* Unsound abstractions around `unsafe` Win32 API calls (`DeviceIoControl`, `CreateFileW`, raw sector writing)
* Parsing vulnerabilities in disk structures, ISO/VTSI headers, or compressed assets

**Please do not report it publicly via GitHub Issues.**

Instead, report it responsibly via one of the following methods:

1. **GitHub Private Vulnerability Advisory:** Navigate to the repository's **Security** tab, click **Advisories**, and select **Report a vulnerability**.
2. **Direct Contact:** Email the maintainer directly at `mmdparsadev@gmail.com` with the subject tag `[SECURITY] rVentoy Vulnerability Report`.

### What to Include

To help us investigate and patch the issue quickly, please provide:

* A clear description of the vulnerability and its potential impact.
* Steps to reproduce the issue (including sample code, payloads, or virtual disk image scenarios).
* Your environment details (Windows build number, target architecture).
* Any proposed fixes or mitigation strategies (optional).

### Response Timeline

* **Initial Acknowledgement:** Within 48 hours of receipt.
* **Assessment & Status Update:** Within 5 business days after acknowledgement.
* **Coordination:** We will collaborate with you to verify the fix and agree upon a public disclosure date once a patched release is published.
