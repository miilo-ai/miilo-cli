import { spawn } from "node:child_process";

/** Use the same native observation engine as the Rust CLI, without a shell. */
export async function runLocal(args: string[]): Promise<void> {
  const executable = process.env.MIILO_LOCAL_EXECUTABLE || "miilo-local";
  await new Promise<void>((resolve, reject) => {
    const child = spawn(executable, args, { stdio: "inherit", shell: false });
    const interrupt = (): void => { child.kill("SIGINT"); };
    process.on("SIGINT", interrupt);
    child.once("error", () => {
      process.removeListener("SIGINT", interrupt);
      reject(new Error("Install the native Miilo CLI (including miilo-local), or set MIILO_LOCAL_EXECUTABLE. Local observation does not require a Miilo API URL."));
    });
    child.once("exit", (code, signal) => {
      process.removeListener("SIGINT", interrupt);
      if (code === 0) resolve();
      else reject(new Error(`Local observation stopped (${signal || code || "unknown"}).`));
    });
  });
}
