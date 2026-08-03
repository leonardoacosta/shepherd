// installed by shepherd
// managed by shepherd; reinstalling or updating the integration overwrites this file.
// add custom hooks/plugins beside this file instead of editing it.
// SHEPHERD_INTEGRATION_ID=pi
// SHEPHERD_INTEGRATION_VERSION=7
// @ts-nocheck

import net from "node:net";

const SHEPHERD_ENV = process.env.SHEPHERD_ENV;
const socketPath = process.env.SHEPHERD_SOCKET_PATH;
const socketEndpoint =
  process.platform === "win32" && socketPath ? `\\\\.\\pipe\\${socketPath}` : socketPath;
const paneId = process.env.SHEPHERD_PANE_ID;
const source = "shepherd:pi";
const workflowLifecycleKey = Symbol.for("pi.workflow.lifecycle.v1");

function enabled() {
  return SHEPHERD_ENV === "1" && !!socketPath && !!paneId;
}

function sendRequestAttempt(request: unknown, timeoutMs: number): Promise<boolean> {
  if (!enabled()) {
    return Promise.resolve(true);
  }

  return new Promise((resolve) => {
    let done = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const finish = (delivered: boolean) => {
      if (done) return;
      done = true;
      if (timeout) {
        clearTimeout(timeout);
      }
      socket.destroy();
      resolve(delivered);
    };

    const socket = net.createConnection(socketEndpoint!);
    socket.on("error", () => finish(false));
    socket.on("connect", () => socket.write(`${JSON.stringify(request)}\n`));
    socket.on("data", () => finish(true));
    socket.on("end", () => finish(false));
    timeout = setTimeout(() => finish(false), timeoutMs);
    timeout.unref?.();
  });
}

async function sendRequest(request: unknown): Promise<void> {
  if (await sendRequestAttempt(request, 500)) {
    return;
  }
  await sendRequestAttempt(request, 1500);
}

type AgentState = "working" | "blocked" | "idle";

type QueuedState = {
  state: AgentState;
  message?: string;
  seq: number;
};

type WorkflowLifecycleSnapshot = {
  version: 1;
  sessionId: string;
  active: true;
  command: "apply" | "apply-all";
  concurrentChanges?: number;
};

type WorkflowLifecycleProvider = {
  snapshot: (sessionId: string) => unknown;
  subscribe: (sessionId: string, onChange: (snapshot: unknown) => void) => unknown;
};

type WorkflowLifecycleBinding = {
  provider: WorkflowLifecycleProvider;
  sessionId: string;
  ready: boolean;
  dispose?: () => void;
};

let reportSeq = Date.now() * 1000;
let currentAgentSessionId: string | undefined;
let currentAgentSessionPath: string | undefined;

function nextReportSeq(): number {
  reportSeq += 1;
  return reportSeq;
}

function updateSessionRef(ctx: any): void {
  try {
    const file = ctx?.sessionManager?.getSessionFile?.();
    currentAgentSessionPath =
      typeof file === "string" && file.startsWith("/") ? file : undefined;
  } catch {
    currentAgentSessionPath = undefined;
  }

  try {
    const id = ctx?.sessionManager?.getSessionId?.();
    currentAgentSessionId = typeof id === "string" && id.length > 0 ? id : undefined;
  } catch {
    currentAgentSessionId = undefined;
  }
}

function withSessionRef(params: Record<string, unknown>): Record<string, unknown> {
  if (currentAgentSessionPath) {
    return { ...params, agent_session_path: currentAgentSessionPath };
  }
  if (currentAgentSessionId) {
    return { ...params, agent_session_id: currentAgentSessionId };
  }
  return params;
}

function currentSessionRef(): Record<string, unknown> | undefined {
  if (currentAgentSessionPath) {
    return { agent_session_path: currentAgentSessionPath };
  }
  if (currentAgentSessionId) {
    return { agent_session_id: currentAgentSessionId };
  }
  return undefined;
}

function reportSession(sessionStartSource?: string): Promise<void> {
  const sessionRef = currentSessionRef();
  if (!sessionRef) {
    return Promise.resolve();
  }

  return sendRequest({
    id: `${source}:session:${Date.now()}:${Math.random().toString(36).slice(2)}`,
    method: "pane.report_agent_session",
    params: {
      pane_id: paneId,
      source,
      agent: "pi",
      seq: nextReportSeq(),
      session_start_source: sessionStartSource,
      ...sessionRef,
    },
  });
}

function sendState(state: AgentState, message?: string, seq = nextReportSeq()): Promise<void> {
  return sendRequest({
    id: `${source}:${Date.now()}:${Math.random().toString(36).slice(2)}`,
    method: "pane.report_agent",
    params: withSessionRef({
      pane_id: paneId,
      source,
      agent: "pi",
      state,
      message,
      seq,
    }),
  });
}

let sendInFlight = false;
let queuedState: QueuedState | undefined;

function queueState(state: AgentState, message?: string): void {
  queuedState = { state, message, seq: nextReportSeq() };
  if (!sendInFlight) {
    void drainStateQueue();
  }
}

async function drainStateQueue(): Promise<void> {
  if (sendInFlight) {
    return;
  }

  sendInFlight = true;
  try {
    while (queuedState) {
      const next = queuedState;
      queuedState = undefined;
      await sendState(next.state, next.message, next.seq);
    }
  } finally {
    sendInFlight = false;
    if (queuedState) {
      void drainStateQueue();
    }
  }
}

export default function (pi) {
  if (!enabled()) {
    return;
  }

  let agentActive = false;
  let blockedCount = 0;
  let blockedMessage: string | undefined;
  let lastState: AgentState | undefined;
  let lastMessage: string | undefined;
  let rootSession = false;
  let workflowSnapshot: WorkflowLifecycleSnapshot | undefined;
  let workflowBinding: WorkflowLifecycleBinding | undefined;

  function validWorkflowSnapshot(
    value: unknown,
    sessionId: string,
  ): value is WorkflowLifecycleSnapshot {
    if (!value || typeof value !== "object") {
      return false;
    }
    const snapshot = value as Partial<WorkflowLifecycleSnapshot>;
    return (
      snapshot.version === 1 &&
      snapshot.sessionId === sessionId &&
      snapshot.active === true &&
      (snapshot.command === "apply" || snapshot.command === "apply-all") &&
      (snapshot.concurrentChanges === undefined ||
        (Number.isFinite(snapshot.concurrentChanges) &&
          Number.isInteger(snapshot.concurrentChanges) &&
          snapshot.concurrentChanges >= 0))
    );
  }

  function currentWorkflowProvider(): WorkflowLifecycleProvider | undefined {
    const provider = (globalThis as any)[workflowLifecycleKey];
    if (
      !provider ||
      typeof provider !== "object" ||
      typeof provider.snapshot !== "function" ||
      typeof provider.subscribe !== "function"
    ) {
      return undefined;
    }
    return provider;
  }

  function disposeWorkflowBinding(): void {
    const binding = workflowBinding;
    workflowBinding = undefined;
    workflowSnapshot = undefined;
    try {
      binding?.dispose?.();
    } catch {
      // Workflow lifecycle consumers are fail-open.
    }
  }

  function refreshWorkflowSnapshot(binding: WorkflowLifecycleBinding, publish: boolean): void {
    if (workflowBinding !== binding) {
      return;
    }
    try {
      const snapshot = binding.provider.snapshot(binding.sessionId);
      workflowSnapshot = validWorkflowSnapshot(snapshot, binding.sessionId)
        ? snapshot
        : undefined;
    } catch {
      workflowSnapshot = undefined;
    }
    if (publish) {
      publishState();
    }
  }

  function bindWorkflowLifecycle(): void {
    disposeWorkflowBinding();
    const provider = currentWorkflowProvider();
    const sessionId = currentAgentSessionId;
    if (!provider || !sessionId) {
      return;
    }

    const binding: WorkflowLifecycleBinding = { provider, sessionId, ready: false };
    workflowBinding = binding;
    try {
      const dispose = provider.subscribe(sessionId, () => {
        if (binding.ready) {
          refreshWorkflowSnapshot(binding, true);
        }
      });
      if (typeof dispose !== "function") {
        disposeWorkflowBinding();
        return;
      }
      binding.dispose = dispose;
      binding.ready = true;
      refreshWorkflowSnapshot(binding, false);
    } catch {
      disposeWorkflowBinding();
    }
  }

  function workflowMessage(snapshot: WorkflowLifecycleSnapshot): string {
    const count =
      snapshot.concurrentChanges === undefined
        ? ""
        : ` · ${snapshot.concurrentChanges} changes`;
    return `Pi · /${snapshot.command}${count}`;
  }

  function desiredState() {
    if (blockedCount > 0) {
      return { state: "blocked" as const, message: blockedMessage };
    }
    if (workflowSnapshot) {
      return { state: "working" as const, message: workflowMessage(workflowSnapshot) };
    }
    if (agentActive) {
      return { state: "working" as const, message: undefined };
    }
    return { state: "idle" as const, message: undefined };
  }

  function publishState(force = false) {
    const next = desiredState();
    if (!force && next.state === lastState && next.message === lastMessage) {
      return;
    }
    lastState = next.state;
    lastMessage = next.message;
    queueState(next.state, next.message);
  }

  pi.events.on("shepherd:blocked", (data) => {
    if (!rootSession) {
      return;
    }
    if (!data?.active) {
      blockedCount = Math.max(0, blockedCount - 1);
      if (blockedCount === 0) {
        blockedMessage = undefined;
        if (workflowBinding) {
          refreshWorkflowSnapshot(workflowBinding, true);
          return;
        }
      }
      publishState();
      return;
    }

    blockedCount += 1;
    blockedMessage = data.label;
    publishState();
  });

  pi.on("session_start", async (event, ctx) => {
    if (ctx?.hasUI !== true) {
      return;
    }
    rootSession = true;
    updateSessionRef(ctx);
    await reportSession(event?.reason);
    bindWorkflowLifecycle();
    // A reload can replace this extension mid-run without emitting another agent_start.
    agentActive = ctx?.isIdle?.() === false;
    publishState(true);
  });

  pi.on("agent_start", (_event, ctx) => {
    if (!rootSession) {
      return;
    }
    updateSessionRef(ctx);
    void reportSession();
    agentActive = true;
    publishState();
  });

  pi.on("agent_settled", (_event, ctx) => {
    if (!rootSession || ctx?.isIdle?.() !== true) {
      return;
    }

    agentActive = false;
    publishState();
  });

  pi.on("session_shutdown", () => {
    if (!rootSession) {
      return;
    }
    rootSession = false;
    disposeWorkflowBinding();
  });
}
