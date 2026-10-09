import type { MyDashboard } from "@/bindings/MyDashboard";
import type { TeamDashboard } from "@/bindings/TeamDashboard";
import { call } from "@/lib/ipc";

export const getTeamDashboard = () => call<TeamDashboard>("dashboard_team");
export const getMyDashboard = () => call<MyDashboard>("dashboard_mine");
