export type Command =
  | { kind: "help" }
  | { kind: "doctor" }
  | { kind: "connections" }
  | { kind: "work-list"; connection?: string }
  | { kind: "work-show"; target: string }
  | { kind: "work-explain"; target: string; question?: string }
  | { kind: "watch" };

export interface Arguments {
  endpoint: string;
  token?: string;
  json: boolean;
  command: Command;
}

export function parseArguments(values: string[], environment = process.env): Arguments {
  const remaining: string[] = [];
  let endpoint = environment.MIILO_URL ?? "http://127.0.0.1:3700";
  let token = environment.MIILO_TOKEN;
  let json = false;
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--json") json = true;
    else if (value === "--endpoint" || value === "--url") {
      endpoint = required(values[++index], `${value} needs a value`);
    }
    else if (value === "--token") token = required(values[++index], "--token needs a value");
    else remaining.push(required(value, "missing argument"));
  }
  const command = parseCommand(remaining);
  return token ? { endpoint, token, json, command } : { endpoint, json, command };
}

function parseCommand(values: string[]): Command {
  if ((values[0] === "--help" || values[0] === "-h") && values.length === 1) return { kind: "help" };
  if (values[0] === "doctor" && values.length === 1) return { kind: "doctor" };
  if (values[0] === "connections" && values[1] === "list" && values.length === 2) return { kind: "connections" };
  if (values[0] === "watch" && values.length === 1) return { kind: "watch" };
  if (values[0] === "work" && values[1] === "list") {
    if (values.length === 2) return { kind: "work-list" };
    if (values[2] === "--connection" && values[3] && values.length === 4) {
      return { kind: "work-list", connection: values[3] };
    }
  }
  if (values[0] === "work" && values[1] === "show" && values[2] && values.length === 3) {
    validateTarget(values[2]);
    return { kind: "work-show", target: values[2] };
  }
  if (values[0] === "work" && values[1] === "explain" && values[2]) {
    validateTarget(values[2]);
    if (values.length === 3) return { kind: "work-explain", target: values[2] };
    if (values[3] === "--question" && values[4] && values.length === 5) {
      return { kind: "work-explain", target: values[2], question: values[4] };
    }
  }
  throw new Error("usage: miilo [--endpoint URL] [--token TOKEN] [--json] <doctor|connections list|work list|work show|work explain|watch>");
}

export function validateTarget(value: string): [string, string] {
  const separator = value.indexOf("/");
  if (separator <= 0 || separator === value.length - 1) {
    throw new Error("work target must be written as <connection>/<work-id>");
  }
  return [value.slice(0, separator), value.slice(separator + 1)];
}

function required(value: string | undefined, message: string): string {
  if (!value) throw new Error(message);
  return value;
}
