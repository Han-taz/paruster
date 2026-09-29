# Error codes

`contracts/errors.json` is the versioned inventory of stable protocol values shared by the Python API and MCP protocol. Code strings are compatibility identifiers: messages may be clarified, but a code's meaning must not be repurposed or silently removed.

All 13 entries in the inventory are reserved protocol values for the migration. Their presence is not a claim that the corresponding failure path is implemented or reachable today. Runtime status belongs in implementation and migration status documentation, not in this frozen identifier list.

Error text crossing Python or MCP boundaries must not expose secrets, native stack traces, or internal implementation details. MCP error responses use text content and mark the result with `isError: true`.
