import { existsSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawn, type ChildProcess } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

import { describe, expect, test } from 'bun:test';

import { HerdrApiClient } from '../src/index';

function createTestBase() {
  return mkdtempSync(path.join(os.tmpdir(), 'herdr-ts-'));
}

function herdrBinPath() {
  return process.env.HERDR_TS_HERDR_BIN ?? path.resolve(import.meta.dir, '..', '..', '..', 'target', 'debug', 'herdr');
}

function spawnHerdrServer(base: string) {
  const configHome = path.join(base, 'config');
  const runtimeDir = path.join(base, 'runtime');
  const socketPath = path.join(runtimeDir, 'herdr.sock');

  mkdirSync(path.join(configHome, 'herdr'), { recursive: true });
  mkdirSync(runtimeDir, { recursive: true });
  writeFileSync(path.join(configHome, 'herdr', 'config.toml'), 'onboarding = false\n');

  const binaryPath = herdrBinPath();
  const useBuiltBinary = existsSync(binaryPath);
  const child = spawn(
    useBuiltBinary ? binaryPath : 'cargo',
    useBuiltBinary ? ['server'] : ['run', '--quiet', '--bin', 'herdr', '--', 'server'],
    {
      cwd: path.resolve(import.meta.dir, '..', '..', '..'),
      env: {
        ...process.env,
        HERDR_CLIENT_SOCKET_PATH: '',
        HERDR_ENV: '',
        HERDR_SOCKET_PATH: socketPath,
        SHELL: '/bin/sh',
        XDG_CONFIG_HOME: configHome,
        XDG_RUNTIME_DIR: runtimeDir,
      },
      stdio: 'ignore',
    },
  );

  return { child, socketPath };
}

async function waitForSocket(socketPath: string, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!existsSync(socketPath)) {
      await delay(25);
      continue;
    }

    const connected = await new Promise<boolean>((resolve) => {
      const socket = net.createConnection(socketPath);
      socket.once('connect', () => {
        socket.destroy();
        resolve(true);
      });
      socket.once('error', () => {
        socket.destroy();
        resolve(false);
      });
    });
    if (connected) {
      return;
    }
    await delay(25);
  }

  throw new Error(`socket did not appear at ${socketPath}`);
}

async function shutdown(child: ChildProcess, base: string) {
  child.kill('SIGKILL');
  await delay(50);
  rmSync(base, { force: true, recursive: true });
}

async function readEventsUntil(
  clientEvents: AsyncIterable<{ event: string }>,
  expected: string[],
  timeoutMs = 5_000,
) {
  const remaining = new Set(expected);
  const events: Array<{ event: string }> = [];
  const deadline = Date.now() + timeoutMs;

  for await (const event of clientEvents) {
    events.push(event);
    remaining.delete(event.event);
    if (remaining.size === 0) {
      return events;
    }
    if (Date.now() >= deadline) {
      break;
    }
  }

  throw new Error(`timed out waiting for events: ${[...remaining].join(', ')}`);
}

describe('HerdrApiClient', () => {
  test('lists panes and streams workspace lifecycle events', async () => {
    const base = createTestBase();
    const { child, socketPath } = spawnHerdrServer(base);

    try {
      await waitForSocket(socketPath);

      const client = new HerdrApiClient({ socketPath });
      const subscription = await client.subscribe([
        { type: 'workspace.created' },
        { type: 'pane.created' },
      ]);

      const created = await client.workspaceCreate({
        cwd: process.cwd(),
        focus: true,
      });
      expect(created.result.type).toBe('workspace_created');

      const panes = await client.paneList({});
      expect(panes.result.type).toBe('pane_list');
      expect(panes.result.panes.length).toBeGreaterThan(0);

      const events = await readEventsUntil(subscription, ['workspace_created', 'pane_created']);
      expect(events.some((event) => event.event === 'workspace_created')).toBe(true);
      expect(events.some((event) => event.event === 'pane_created')).toBe(true);
    } finally {
      await shutdown(child, base);
    }
  });
});
