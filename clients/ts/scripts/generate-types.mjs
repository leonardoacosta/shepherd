import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

import { compile } from 'json-schema-to-typescript';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageDir = path.resolve(scriptDir, '..');
const repoRoot = path.resolve(packageDir, '..', '..');
const schemaPath = path.join(repoRoot, 'docs', 'next', 'api', 'shepherd-api.schema.json');
const outputPath = path.join(packageDir, 'src', 'generated', 'schema.ts');
const checkMode = process.argv.includes('--check');

const header = [
  '// Generated from docs/next/api/shepherd-api.schema.json.',
  '// Do not edit by hand.',
  '',
].join('\n');

const schemaDocument = JSON.parse(await readFile(schemaPath, 'utf8'));
const schemaEntries = [
  ['request', 'Request'],
  ['success_response', 'SuccessResponse'],
  ['error_response', 'ErrorResponse'],
  ['event', 'EventEnvelope'],
  ['subscription_event', 'SubscriptionEventEnvelope'],
];

function normalizeSchema(schemaName, value) {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeSchema(schemaName, item));
  }

  if (!value || typeof value !== 'object') {
    return value;
  }

  const normalized = {};
  for (const [key, child] of Object.entries(value)) {
    if (key === '$ref' && typeof child === 'string') {
      normalized[key] = child.replace(`#/schemas/${schemaName}/`, '#/');
      continue;
    }
    normalized[key] = normalizeSchema(schemaName, child);
  }
  return normalized;
}

const generatedSections = [];
for (const [schemaName, typeName] of schemaEntries) {
  const normalizedSchema = normalizeSchema(schemaName, schemaDocument.schemas[schemaName]);
  const generatedSection = await compile(normalizedSchema, typeName, {
    additionalProperties: false,
    bannerComment: '',
    declareExternallyReferenced: false,
    style: {
      semi: true,
      singleQuote: true,
    },
    unreachableDefinitions: true,
    unknownAny: false,
  });
  generatedSections.push(generatedSection.trim());
}

const generatedText = `${header}${generatedSections.join('\n\n')}\n`;

if (checkMode) {
  let existing = '';
  try {
    existing = await readFile(outputPath, 'utf8');
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
      console.error(`generated schema types are missing: ${outputPath}`);
      process.exit(1);
    }
    throw error;
  }

  if (existing !== generatedText) {
    console.error(`generated schema types drifted: ${outputPath}`);
    process.exit(1);
  }

  console.log('generated schema types are up to date');
  process.exit(0);
}

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, generatedText);
console.log(`wrote ${path.relative(repoRoot, outputPath)}`);
