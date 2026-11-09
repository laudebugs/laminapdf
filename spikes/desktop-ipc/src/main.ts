import { invoke } from "@tauri-apps/api/core";
import { type Plan, runBench } from "./bench";
import { runCoi } from "./coi";

const plan = await invoke<Plan>("plan");
if (plan.mode === "coi") await runCoi();
else await runBench(plan);
