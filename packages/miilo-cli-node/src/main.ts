#!/usr/bin/env node
import { parseArguments, validateTarget } from "./args.js";
import { MiiloClient, MiiloHttpError } from "./client.js";
import { writeConnections, writeDoctor, writeExplanation, writeJson, writeStreamEvent, writeWork, writeWorkItem } from "./output.js";
import type { ExplanationRequest, WorkItem } from "./types.js";

export async function run(values = process.argv.slice(2)): Promise<void> {
  const args = parseArguments(values);
  const client = new MiiloClient(args.endpoint, args.token);
  if (args.command.kind === "doctor") {
    const [health, snapshot] = await Promise.all([client.health(), client.snapshot()]);
    return args.json ? writeJson({ health, snapshot }) : writeDoctor(health, snapshot);
  }
  if (args.command.kind === "connections") {
    const response = await client.connections();
    return args.json ? writeJson(response) : writeConnections(response);
  }
  if (args.command.kind === "work-list") {
    const snapshot = await client.snapshot();
    const connection = args.command.connection;
    const work = snapshot.workItems.filter((item) => !connection || item.providerId === connection);
    return args.json ? writeJson(work) : writeWork(work);
  }
  if (args.command.kind === "work-show") {
    const item = await findWork(client, args.command.target);
    return args.json ? writeJson(item) : writeWorkItem(item);
  }
  if (args.command.kind === "work-explain") {
    const item = await findWork(client, args.command.target);
    const request: ExplanationRequest = {
      providerId: item.providerId,
      workItemId: item.id,
      expectedRevision: item.revision,
      preferences: { detail: "normal", lead: "cause", includeTechnicalDetails: false, speakSensitiveValues: false },
      ...(args.command.question ? { question: args.command.question } : {}),
    };
    const bundle = await client.explain(request);
    return args.json ? writeJson(bundle) : writeExplanation(bundle);
  }
  await watch(client, args.json);
}

async function findWork(client: MiiloClient, target: string): Promise<WorkItem> {
  const [providerId, workItemId] = validateTarget(target);
  const item = (await client.snapshot()).workItems.find((candidate) => candidate.providerId === providerId && candidate.id === workItemId);
  if (!item) throw new Error(`work item ${target} was not found`);
  return item;
}

async function watch(client: MiiloClient, json: boolean): Promise<void> {
  let cursor: string | undefined;
  let backoff = 1000;
  while (true) {
    try {
      for await (const event of client.stream(cursor)) {
        backoff = 1000;
        if (event.type === "event") cursor = event.streamCursor;
        json ? writeJson(event, true) : writeStreamEvent(event);
      }
    } catch (error) {
      if (error instanceof MiiloHttpError && [401, 403].includes(error.status)) throw error;
      if (error instanceof MiiloHttpError && error.status === 409) {
        cursor = (await client.snapshot()).cursor;
        continue;
      }
      console.error(`Miilo stream unavailable: ${message(error)}; retrying in ${backoff / 1000}s`);
      await new Promise((resolve) => setTimeout(resolve, backoff));
      backoff = Math.min(backoff * 2, 30_000);
    }
  }
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

if (import.meta.url === new URL(process.argv[1] ?? "", "file:").href) {
  run().catch((error: unknown) => {
    console.error(`miilo: ${message(error)}`);
    process.exitCode = 1;
  });
}
