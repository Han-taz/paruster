# Authored option projection captures

The five Markdown inputs in `captured.json` are authored CC0 fixtures. Answers were captured locally on 2026-09-30 from the read-only migration oracle's `script-tags.ts`, `plain-markdown.ts`, and `html-tables.ts`; their SHA-256 pins are inside the file. These are pure helper observations, not successful document-parser captures, and do not advance the protected document-manifest numerator.

The capture used Node.js 22.22.0's `node:module.stripTypeScriptTypes` to erase types in memory, imported the resulting helper modules from data URLs, and rewired only the plain helper's script-tags import to that in-memory module. No oracle file was modified or copied into Git. Inputs were passed unchanged to `toPlainMarkdown`, `toHtmlTables`, their ordered composition, and `stripScriptTags` followed by plain. Expected strings are compared exactly without normalization.

Node, the oracle checkout, and capture tooling are not required by builds, tests, packaging or runtime. Rust tests load only these authored inputs and their recorded outputs. Cases include Unicode script values, escaped bold/link syntax, escaped table pipes, allowed inline HTML, nested tables and option ordering.
