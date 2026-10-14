export const PROTOCOL_VERSION = "0.1" as const;

export interface HealthResponse {
  ok: boolean;
  service: string;
  protocolVersion: string;
  providers: number;
  identity: boolean;
  storage: boolean;
}

export interface ConnectionSummary {
  id: string;
  name: string;
  adapter: string;
  enabled: boolean;
  credentialConfigured: boolean;
}

export interface ConnectionsResponse {
  connections: ConnectionSummary[];
}

export interface WorkItem {
  id: string;
  providerId: string;
  title: string;
  summary: string;
  lifecycle: string;
  phase: string;
  attention: string;
  health: string;
  revision: string;
}

export interface ObservatoryEvent {
  protocolVersion: string;
  eventId: string;
  providerId: string;
  occurredAt: string;
  kind: string;
  summary: string;
}

export interface ObservatorySnapshot {
  protocolVersion: string;
  snapshotId: string;
  cursor: string;
  providers: unknown[];
  workItems: WorkItem[];
  attention: unknown[];
}

export interface ExplanationRequest {
  providerId: string;
  workItemId: string;
  expectedRevision: string;
  question?: string;
  preferences: {
    detail: "normal";
    lead: "cause";
    includeTechnicalDetails: false;
    speakSensitiveValues: false;
  };
}

export interface ExplanationBundle {
  protocolVersion: string;
  title: string;
  spokenSummary: string;
  confidence: string;
  recommendation: string;
}

export type StreamEvent =
  | { type: "event"; streamCursor: string; event: ObservatoryEvent }
  | { type: "error"; reason: string; message: string };
