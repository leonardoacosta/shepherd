import { readFileSync } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';

import { describe, expect, test } from 'bun:test';

const packageDir = path.resolve(import.meta.dir, '..');
const generatedPath = path.join(packageDir, 'src', 'generated', 'schema.ts');

describe('schema generation', () => {
  test('committed generated schema types match the checked-in API schema', () => {
    const result = spawnSync(process.execPath, ['run', 'generate:types:check'], {
      cwd: packageDir,
      encoding: 'utf8',
    });

    expect(result.status).toBe(0);
    const generated = readFileSync(generatedPath, 'utf8');
    expect(generated).toContain('export type Request =');
    expect(generated).toContain('export interface SuccessResponse');
    expect(generated).toContain('export interface EventEnvelope');
  });
});
