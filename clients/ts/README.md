# `@herdr/api-client`

Reference TypeScript client for Herdr's local newline-delimited JSON API.

Regenerate the checked-in schema bindings from the committed API schema with:

```bash
cd clients/ts
bun run generate:types
```

Verify the bindings are in sync and run the smoke tests with:

```bash
cd clients/ts
bun run generate:types:check
bun test
```
