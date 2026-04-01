# Security Policy

## Supported Versions

Only the latest release receives security fixes.

## Reporting a Vulnerability

Please do **not** open a public GitHub issue for security vulnerabilities.

Use GitHub's private vulnerability reporting instead: go to the [Security tab](https://github.com/korti11/satisfactory-pathfinder/security) of this repository and click **Report a vulnerability**. You will receive a response as soon as possible.

## Scope

The following are considered security issues:

- Path traversal via `--data-dir` or `--factory` flags allowing reads outside the intended directory
- Malicious content in factory JSON files (`--factory`) causing unintended behaviour
- Any input that causes unintended code execution

## Out of Scope

The following are **not** considered security issues:

- Incorrect or missing game data (open a regular bug report instead)
- Crashes or panics caused by malformed input that have no security impact
- Missing features
