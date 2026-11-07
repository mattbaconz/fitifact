import type { DiscordCap } from "../../../web/src/lib/destinations";
import { fillPlans, savePlan } from "./sites";

const plan = document.getElementById("plan") as HTMLSelectElement;
plan.addEventListener("change", () => void savePlan(plan.value as DiscordCap));
document.getElementById("open")!.addEventListener("click", () => {
  void chrome.tabs.create({ url: chrome.runtime.getURL("panel.html") });
  window.close();
});
void fillPlans(plan);
