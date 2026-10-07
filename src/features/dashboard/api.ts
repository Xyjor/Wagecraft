import type { TeamDashboard } from "@/bindings/TeamDashboard";
import { call } from "@/lib/ipc";

export const getTeamDashboard = () => call<TeamDashboard>("dashboard_team");
