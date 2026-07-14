# EULYNX context

EULYNX standardizes interfaces used in railway signalling, including the
communication context around object controllers. RaSTA is relevant as a safe
transport layer for EULYNX-style communication between signalling systems and
object controllers.

The external SBB RaSTA stack README states that SBB used that stack for a proof
of concept of a EULYNX Object Controller. This repository's controlled SBB
interoperability evidence therefore provides useful engineering context, but it
does not establish EULYNX or SCI compliance.

## Relationship to this project

The `signal-controller` and `interlocking-controller` applications are
simplified educational object-controller examples. They demonstrate
bidirectional application messages through the public RaSTA endpoint API,
including connection management, polling, data exchange, diagnostics, tracing,
and graceful close.

The examples do not implement full EULYNX SCI/PDI behavior. They are not EULYNX
or SCI compliant, independently assessed, certified, certification-ready, or
production-ready. Their application messages and profiles are test-only and do
not represent an operational railway interface specification.

Related documentation:

- [Signal/interlocking example](signal-interlocking-example.md)
- [Final controlled interop summary](final-interop-summary.md)
- [Docker/Podman interop reproduction](docker-interop.md)
