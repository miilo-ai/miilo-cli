import type { ConnectionsResponse, ExplanationBundle, HealthResponse, ObservatorySnapshot, StreamEvent, WorkItem } from "./types.js";

export function writeJson(value: unknown, line = false): void {
  process.stdout.write(`${JSON.stringify(value, undefined, line ? undefined : 2)}\n`);
}

export function writeDoctor(health: HealthResponse, snapshot: ObservatorySnapshot): void {
  console.log(`Miilo API: ready (protocol ${health.protocolVersion})`);
  console.log(`Providers: ${snapshot.providers.length}`);
  console.log(`Work items: ${snapshot.workItems.length}`);
  console.log(`Attention items: ${snapshot.attention.length}`);
  console.log(`Identity: ${health.identity ? "configured" : "development mode"}`);
  console.log(`Storage: ${health.storage ? "configured" : "development mode"}`);
}

export function writeConnections(response: ConnectionsResponse): void {
  if (response.connections.length === 0) return console.log("No connections configured.");
  for (const item of response.connections) {
    console.log(`${clean(item.id)}\t${clean(item.name)}\t${clean(item.adapter)}\t${item.enabled ? "enabled" : "disabled"}`);
  }
}

export function writeWork(items: WorkItem[]): void {
  if (items.length === 0) return console.log("No work items found.");
  for (const item of items) {
    console.log(`${clean(item.providerId)}/${clean(item.id)}\t${clean(item.lifecycle)}\t${clean(item.phase)}\t${clean(item.title)}`);
  }
}

export function writeWorkItem(item: WorkItem): void {
  console.log(clean(item.title));
  console.log(`ID: ${clean(item.providerId)}/${clean(item.id)}`);
  console.log(`State: ${clean(item.lifecycle)} / ${clean(item.phase)}`);
  console.log(`Health: ${clean(item.health)}`);
  console.log(`Attention: ${clean(item.attention)}`);
  console.log(`Revision: ${clean(item.revision)}`);
  console.log(clean(item.summary));
}

export function writeExplanation(bundle: ExplanationBundle): void {
  console.log(clean(bundle.title));
  console.log(clean(bundle.spokenSummary));
  console.log(`Confidence: ${clean(bundle.confidence)}`);
  if (bundle.recommendation) console.log(`Recommendation: ${clean(bundle.recommendation)}`);
}

export function writeStreamEvent(value: StreamEvent): void {
  if (value.type === "error") return console.error(`${clean(value.reason)}: ${clean(value.message)}`);
  console.log(`${clean(value.event.occurredAt)}\t${clean(value.event.providerId)}\t${clean(value.event.kind)}\t${clean(value.event.summary)}`);
}

function clean(value: string): string {
  return value.replace(/[\u0000-\u001f\u007f-\u009f]/gu, " ");
}
