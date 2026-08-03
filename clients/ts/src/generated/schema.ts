// Generated from docs/next/api/shepherd-api.schema.json.
// Do not edit by hand.
export type Request = {
  id: string;
} & (
  | {
      method: 'ping';
      params: PingParams;
    }
  | {
      method: 'server.stop';
      params: EmptyParams;
    }
  | {
      method: 'server.live_handoff';
      params: ServerLiveHandoffParams;
    }
  | {
      method: 'server.reload_config';
      params: EmptyParams;
    }
  | {
      method: 'server.agent_manifests';
      params: EmptyParams;
    }
  | {
      method: 'server.reload_agent_manifests';
      params: EmptyParams;
    }
  | {
      method: 'notification.show';
      params: NotificationShowParams;
    }
  | {
      method: 'client.window_title.set';
      params: ClientWindowTitleSetParams;
    }
  | {
      method: 'client.window_title.clear';
      params: EmptyParams;
    }
  | {
      method: 'session.snapshot';
      params: EmptyParams;
    }
  | {
      method: 'workspace.create';
      params: WorkspaceCreateParams;
    }
  | {
      method: 'workspace.list';
      params: EmptyParams;
    }
  | {
      method: 'workspace.get';
      params: WorkspaceTarget;
    }
  | {
      method: 'workspace.focus';
      params: WorkspaceTarget;
    }
  | {
      method: 'workspace.rename';
      params: WorkspaceRenameParams;
    }
  | {
      method: 'workspace.move';
      params: WorkspaceMoveParams;
    }
  | {
      method: 'workspace.move_block';
      params: WorkspaceMoveBlockParams;
    }
  | {
      method: 'workspace.report_metadata';
      params: WorkspaceReportMetadataParams;
    }
  | {
      method: 'workspace.close';
      params: WorkspaceTarget;
    }
  | {
      method: 'worktree.list';
      params: WorktreeListParams;
    }
  | {
      method: 'worktree.create';
      params: WorktreeCreateParams;
    }
  | {
      method: 'worktree.open';
      params: WorktreeOpenParams;
    }
  | {
      method: 'worktree.remove';
      params: WorktreeRemoveParams;
    }
  | {
      method: 'tab.create';
      params: TabCreateParams;
    }
  | {
      method: 'tab.list';
      params: TabListParams;
    }
  | {
      method: 'tab.get';
      params: TabTarget;
    }
  | {
      method: 'tab.focus';
      params: TabTarget;
    }
  | {
      method: 'tab.rename';
      params: TabRenameParams;
    }
  | {
      method: 'tab.move';
      params: TabMoveParams;
    }
  | {
      method: 'tab.close';
      params: TabTarget;
    }
  | {
      method: 'agent.list';
      params: EmptyParams;
    }
  | {
      method: 'agent.get';
      params: AgentTarget;
    }
  | {
      method: 'agent.read';
      params: AgentReadParams;
    }
  | {
      method: 'agent.explain';
      params: AgentTarget;
    }
  | {
      method: 'agent.send_keys';
      params: AgentSendKeysParams;
    }
  | {
      method: 'agent.rename';
      params: AgentRenameParams;
    }
  | {
      method: 'agent.view.set';
      params: AgentViewSetParams;
    }
  | {
      method: 'agent.view.clear';
      params: AgentViewClearParams;
    }
  | {
      method: 'agent.focus';
      params: AgentTarget;
    }
  | {
      method: 'agent.start';
      params: AgentStartParams;
    }
  | {
      method: 'agent.prompt';
      params: AgentPromptParams;
    }
  | {
      method: 'agent.wait';
      params: AgentWaitParams;
    }
  | {
      method: 'pane.split';
      params: PaneSplitParams;
    }
  | {
      method: 'pane.swap';
      params: PaneSwapParams;
    }
  | {
      method: 'pane.move';
      params: PaneMoveParams;
    }
  | {
      method: 'pane.zoom';
      params: PaneZoomParams;
    }
  | {
      method: 'pane.layout';
      params: PaneLayoutParams;
    }
  | {
      method: 'pane.process_info';
      params: PaneProcessInfoParams;
    }
  | {
      method: 'layout.export';
      params: LayoutExportParams;
    }
  | {
      method: 'layout.apply';
      params: LayoutApplyParams;
    }
  | {
      method: 'layout.set_split_ratio';
      params: LayoutSetSplitRatioParams;
    }
  | {
      method: 'pane.neighbor';
      params: PaneNeighborParams;
    }
  | {
      method: 'pane.edges';
      params: PaneEdgesParams;
    }
  | {
      method: 'pane.focus_direction';
      params: PaneFocusDirectionParams;
    }
  | {
      method: 'pane.resize';
      params: PaneResizeParams;
    }
  | {
      method: 'pane.list';
      params: PaneListParams;
    }
  | {
      method: 'pane.current';
      params: PaneCurrentParams;
    }
  | {
      method: 'pane.get';
      params: PaneTarget;
    }
  | {
      method: 'pane.focus';
      params: PaneTarget;
    }
  | {
      method: 'pane.rename';
      params: PaneRenameParams;
    }
  | {
      method: 'pane.send_text';
      params: PaneSendTextParams;
    }
  | {
      method: 'pane.send_keys';
      params: PaneSendKeysParams;
    }
  | {
      method: 'pane.send_input';
      params: PaneSendInputParams;
    }
  | {
      method: 'pane.read';
      params: PaneReadParams;
    }
  | {
      method: 'pane.graphics.set';
      params: PaneGraphicsSetParams;
    }
  | {
      method: 'pane.graphics.clear';
      params: PaneGraphicsClearParams;
    }
  | {
      method: 'pane.graphics.info';
      params: PaneTarget;
    }
  | {
      method: 'pane.report_agent';
      params: PaneReportAgentParams;
    }
  | {
      method: 'pane.report_agent_session';
      params: PaneReportAgentSessionParams;
    }
  | {
      method: 'pane.report_metadata';
      params: PaneReportMetadataParams;
    }
  | {
      method: 'pane.clear_agent_authority';
      params: PaneClearAgentAuthorityParams;
    }
  | {
      method: 'pane.release_agent';
      params: PaneReleaseAgentParams;
    }
  | {
      method: 'pane.close';
      params: PaneTarget;
    }
  | {
      method: 'popup.close';
      params: EmptyParams;
    }
  | {
      method: 'events.subscribe';
      params: EventsSubscribeParams;
    }
  | {
      method: 'events.wait';
      params: EventsWaitParams;
    }
  | {
      method: 'pane.wait_for_output';
      params: PaneWaitForOutputParams;
    }
  | {
      method: 'integration.install';
      params: IntegrationInstallParams;
    }
  | {
      method: 'integration.uninstall';
      params: IntegrationUninstallParams;
    }
  | {
      method: 'plugin.link';
      params: PluginLinkParams;
    }
  | {
      method: 'plugin.list';
      params: PluginListParams;
    }
  | {
      method: 'plugin.unlink';
      params: PluginUnlinkParams;
    }
  | {
      method: 'plugin.enable';
      params: PluginSetEnabledParams;
    }
  | {
      method: 'plugin.disable';
      params: PluginSetEnabledParams;
    }
  | {
      method: 'plugin.action.list';
      params: PluginActionListParams;
    }
  | {
      method: 'plugin.action.invoke';
      params: PluginActionInvokeParams;
    }
  | {
      method: 'plugin.log.list';
      params: PluginLogListParams;
    }
  | {
      method: 'plugin.pane.open';
      params: PluginPaneOpenParams;
    }
  | {
      method: 'plugin.pane.focus';
      params: PluginPaneFocusParams;
    }
  | {
      method: 'plugin.pane.close';
      params: PluginPaneCloseParams;
    }
);

export interface SuccessResponse {
  id: string;
  result: ResponseResult;
}

export interface ErrorResponse {
  error: ErrorBody;
  id: string;
}

export interface EventEnvelope {
  data: EventData;
  event: EventKind;
}

export interface SubscriptionEventEnvelope {
  data: SubscriptionEventData;
  event: SubscriptionEventKind;
}
